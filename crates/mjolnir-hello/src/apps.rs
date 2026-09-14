//! Bounded server-side mini-app manifest discovery.
//!
//! This module is the only place allowed to contact app publishers. Transport
//! is pinned to the directory record's literal socket address; the record host
//! remains in the URL solely for HTTP Host and TLS SNI. Results are sanitized
//! presentation data stored in memory for `/api/apps`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use rand::Rng;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::routes::DirectoryCache;

const MANIFEST_PATH: &str = "/.well-known/mesh-app.json";
const MAX_MANIFEST_BYTES: usize = 128 * 1024;
const MAX_ICON_BYTES: usize = 64 * 1024;
const MAX_APPS: usize = 64;
const FETCH_TIMEOUT: Duration = Duration::from_secs(3);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
const REFRESH_JITTER_MAX: Duration = Duration::from_secs(30);
const LAST_GOOD_TTL: Duration = Duration::from_secs(60 * 60);
const DIRECTORY_POLL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Deserialize)]
struct DirectoryService {
    name: String,
    ip: String,
    port: u16,
    protocol: String,
    #[serde(default)]
    txt: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct DirectoryProjection {
    #[serde(default)]
    services: Vec<DirectoryService>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
struct AppKey {
    name: String,
    protocol: String,
    ip: String,
    port: u16,
}

impl From<&DirectoryService> for AppKey {
    fn from(service: &DirectoryService) -> Self {
        Self {
            name: service.name.clone(),
            protocol: service.protocol.clone(),
            ip: service.ip.clone(),
            port: service.port,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Manifest {
    v: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    embed: EmbedMode,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_css_px"
    )]
    height: Option<f64>,
}

/// Emit whole-pixel heights as JSON integers (`120`, not `120.0`) so
/// `/api/apps` matches the browser contract's output and the shared fixtures
/// byte-for-byte; fractional heights stay floats, as `clampHeight` keeps them.
fn serialize_css_px<S: serde::Serializer>(value: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(px) if px.fract() == 0.0 => s.serialize_i64(*px as i64),
        Some(px) => s.serialize_f64(*px),
        None => s.serialize_none(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum EmbedMode {
    Card,
    Link,
}

#[derive(Clone, Debug)]
struct CacheEntry {
    manifest: Option<Manifest>,
    fetched_at_unix_ms: u64,
    last_good_at: Option<Instant>,
    next_refresh: Instant,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct AppsResponse {
    version: u8,
    apps: Vec<AppResponse>,
}

#[derive(Debug, Serialize)]
struct AppResponse {
    name: String,
    protocol: String,
    ip: String,
    port: u16,
    manifest: Option<Manifest>,
    fetched_at_unix_ms: u64,
    stale: bool,
    error: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct AppCache {
    entries: Mutex<HashMap<AppKey, CacheEntry>>,
}

impl AppCache {
    fn response(&self, now: Instant) -> AppsResponse {
        let entries = self.entries.lock().expect("app cache poisoned");
        let mut apps: Vec<_> = entries
            .iter()
            .map(|(key, entry)| {
                let expired = entry
                    .last_good_at
                    .is_some_and(|at| now.saturating_duration_since(at) > LAST_GOOD_TTL);
                AppResponse {
                    name: key.name.clone(),
                    protocol: key.protocol.clone(),
                    ip: key.ip.clone(),
                    port: key.port,
                    manifest: if expired {
                        None
                    } else {
                        entry.manifest.clone()
                    },
                    fetched_at_unix_ms: entry.fetched_at_unix_ms,
                    stale: expired || entry.error.is_some(),
                    error: entry.error.clone(),
                }
            })
            .collect();
        apps.sort_by(|a, b| {
            (&a.name, &a.protocol, &a.ip, a.port).cmp(&(&b.name, &b.protocol, &b.ip, b.port))
        });
        AppsResponse { version: 1, apps }
    }

    pub(crate) fn response_json(&self) -> String {
        serde_json::to_string(&self.response(Instant::now())).expect("app cache serializes")
    }
}

static APP_CACHE: OnceLock<AppCache> = OnceLock::new();

pub(crate) fn global_cache() -> &'static AppCache {
    APP_CACHE.get_or_init(AppCache::default)
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn clamp_height(value: f64) -> f64 {
    value.clamp(120.0, 640.0)
}

fn valid_data_image(uri: &str) -> bool {
    if uri.len() > MAX_ICON_BYTES {
        return false;
    }
    let Some((metadata, payload)) = uri.split_once(',') else {
        return false;
    };
    if !matches!(
        metadata.to_ascii_lowercase().as_str(),
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/webp;base64"
            | "data:image/svg+xml;base64"
    ) {
        return false;
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .is_ok()
}

fn manifest_from_value(mut value: Value) -> Option<Manifest> {
    if let Value::String(encoded) = &value {
        value = serde_json::from_str(encoded).ok()?;
    }
    let object = value.as_object()?;
    if object.get("v")?.as_u64() != Some(1) {
        return None;
    }
    let name = match object.get("name") {
        Some(Value::String(value)) => Some(truncate_chars(value, 40)),
        Some(_) => return None,
        None => None,
    };
    let description = match object.get("description") {
        Some(Value::String(value)) => Some(truncate_chars(value, 140)),
        Some(_) => return None,
        None => None,
    };
    let icon = match object.get("icon") {
        Some(Value::String(value)) if valid_data_image(value) => Some(value.clone()),
        Some(_) => return None,
        None => None,
    };
    let embed = match object.get("embed") {
        Some(Value::String(value)) if value == "card" => EmbedMode::Card,
        Some(Value::String(value)) if value == "link" => EmbedMode::Link,
        Some(_) => return None,
        None => EmbedMode::Link,
    };
    let height = match object.get("height") {
        Some(Value::Number(value)) => {
            let value = value.as_f64()?;
            if !value.is_finite() {
                return None;
            }
            Some(clamp_height(value))
        }
        Some(_) => return None,
        None => None,
    };
    Some(Manifest {
        v: 1,
        name,
        description,
        icon,
        embed,
        height,
    })
}

fn valid_relative_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains("//")
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && !path.split('/').any(|part| {
            part.split_once(':').is_some_and(|(scheme, _)| {
                let mut chars = scheme.chars();
                chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
            })
        })
}

fn is_mdns_service_name(name: &str) -> bool {
    let mut parts = name.rsplitn(3, '.');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(transport), Some(protocol), Some(base))
            if transport.starts_with('_')
                && transport.len() > 1
                && protocol.starts_with('_')
                && protocol.len() > 1
                && !base.is_empty()
    )
}

fn app_host(service: &DirectoryService) -> String {
    if is_mdns_service_name(&service.name) {
        match service.ip.parse::<IpAddr>() {
            Ok(IpAddr::V6(_)) => format!("[{}]", service.ip),
            _ => service.ip.clone(),
        }
    } else {
        format!("{}.mesh", service.name)
    }
}

fn is_mini_app(service: &DirectoryService) -> bool {
    matches!(
        service.protocol.to_ascii_lowercase().as_str(),
        "http" | "https"
    ) && service.txt.get("app").map(String::as_str) == Some("v1")
        && service
            .txt
            .get("path")
            .map(String::as_str)
            .is_none_or(valid_relative_path)
        && !service.name.is_empty()
        && service.ip.parse::<IpAddr>().is_ok()
}

fn in_mesh_range(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            octets[0] == 10 && matches!(octets[1], 42 | 254)
        }
        IpAddr::V6(_) => false,
    }
}

/// Certificate-chain and name checks are intentionally bypassed only for
/// presentation-data fetches. Handshake signatures are still verified with
/// ring, proving the peer owns the certificate's private key.
#[derive(Debug)]
struct PresentationOnlyCertificateVerifier(Arc<CryptoProvider>);

impl ServerCertVerifier for PresentationOnlyCertificateVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

struct ManifestFetcher {
    allow_ip: Arc<dyn Fn(IpAddr) -> bool + Send + Sync>,
    timeout: Duration,
}

impl ManifestFetcher {
    fn production() -> Self {
        Self {
            allow_ip: Arc::new(in_mesh_range),
            timeout: FETCH_TIMEOUT,
        }
    }

    #[cfg(test)]
    fn for_tests(
        allow_ip: impl Fn(IpAddr) -> bool + Send + Sync + 'static,
        timeout: Duration,
    ) -> Self {
        Self {
            allow_ip: Arc::new(allow_ip),
            timeout,
        }
    }

    fn agent(&self, socket: SocketAddr, timeout: Duration) -> ureq::Agent {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let tls = rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()
            .expect("ring supports rustls default protocol versions")
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PresentationOnlyCertificateVerifier(
                provider,
            )))
            .with_no_client_auth();

        // AgentBuilder has no ambient proxy unless one is explicitly supplied;
        // the `proxy-from-env` and cookie features are disabled in Cargo.toml.
        ureq::AgentBuilder::new()
            .resolver(move |_netloc: &str| Ok(vec![socket]))
            .redirects(0)
            .timeout_connect(timeout)
            .timeout_read(timeout)
            .timeout_write(timeout)
            .timeout(timeout)
            .tls_config(Arc::new(tls))
            .build()
    }

    fn get(
        &self,
        service: &DirectoryService,
        path: &str,
        limit: usize,
        timeout: Duration,
    ) -> Result<(Vec<u8>, Option<String>), String> {
        let ip: IpAddr = service.ip.parse().map_err(|_| "invalid record IP")?;
        if !(self.allow_ip)(ip) {
            return Err("record IP is outside mesh ranges".to_string());
        }
        let socket = SocketAddr::new(ip, service.port);
        let protocol = service.protocol.to_ascii_lowercase();
        let host = app_host(service);
        let default_port = if protocol == "https" { 443 } else { 80 };
        let authority = if service.port == default_port {
            host.clone()
        } else {
            format!("{host}:{}", service.port)
        };
        let url = format!("{protocol}://{authority}{path}");
        let response = self
            .agent(socket, timeout)
            .get(&url)
            .set("Host", &host)
            .set("Accept-Encoding", "identity")
            .call()
            .map_err(|err| format!("request failed: {err}"))?;
        if response.status() != 200 {
            return Err(format!("unexpected HTTP status {}", response.status()));
        }
        let content_type = response.header("Content-Type").map(str::to_string);
        let mut body = Vec::with_capacity(limit.min(16 * 1024));
        response
            .into_reader()
            .take((limit + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|err| format!("body read failed: {err}"))?;
        if body.len() > limit {
            return Err(format!("response body exceeds {limit} bytes"));
        }
        Ok((body, content_type))
    }

    fn fetch(&self, service: &DirectoryService) -> Result<Manifest, String> {
        let started = Instant::now();
        let (body, _) = self.get(service, MANIFEST_PATH, MAX_MANIFEST_BYTES, self.timeout)?;
        let mut value: Value =
            serde_json::from_slice(&body).map_err(|_| "invalid manifest JSON")?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| "manifest is not an object".to_string())?;

        if let Some(icon) = object.get("icon") {
            let path = icon
                .as_str()
                .filter(|path| valid_relative_path(path))
                .ok_or_else(|| "invalid icon path".to_string())?
                .to_string();
            let remaining = self
                .timeout
                .checked_sub(started.elapsed())
                .filter(|remaining| !remaining.is_zero())
                .ok_or_else(|| "manifest operation timed out".to_string())?;
            let (bytes, content_type) = self.get(service, &path, MAX_ICON_BYTES, remaining)?;
            let mime = content_type
                .as_deref()
                .and_then(|value| value.split(';').next())
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .filter(|value| {
                    matches!(
                        value.as_str(),
                        "image/png" | "image/jpeg" | "image/webp" | "image/svg+xml"
                    )
                })
                .ok_or_else(|| "unsupported icon content type".to_string())?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            object.insert(
                "icon".to_string(),
                Value::String(format!("data:{mime};base64,{encoded}")),
            );
        }

        manifest_from_value(value).ok_or_else(|| "invalid manifest fields".to_string())
    }
}

struct Refresher {
    fetcher: ManifestFetcher,
}

impl Refresher {
    fn production() -> Self {
        Self {
            fetcher: ManifestFetcher::production(),
        }
    }

    fn refresh(&self, directory_json: &str, cache: &AppCache) {
        let Ok(directory) = serde_json::from_str::<DirectoryProjection>(directory_json) else {
            return;
        };
        let services: Vec<_> = directory
            .services
            .into_iter()
            .filter(is_mini_app)
            .take(MAX_APPS)
            .collect();
        let current: HashSet<AppKey> = services.iter().map(AppKey::from).collect();
        let now = Instant::now();
        {
            let mut entries = cache.entries.lock().expect("app cache poisoned");
            entries.retain(|key, _| current.contains(key));
            for key in &current {
                entries.entry(key.clone()).or_insert_with(|| CacheEntry {
                    manifest: None,
                    fetched_at_unix_ms: 0,
                    last_good_at: None,
                    next_refresh: now,
                    error: Some("manifest refresh pending".to_string()),
                });
            }
        }

        for service in services {
            let key = AppKey::from(&service);
            let due = {
                let entries = cache.entries.lock().expect("app cache poisoned");
                entries
                    .get(&key)
                    .is_none_or(|entry| now >= entry.next_refresh)
            };
            if !due {
                continue;
            }

            let result = self.fetcher.fetch(&service);
            let jitter =
                Duration::from_secs(rand::rng().random_range(0..=REFRESH_JITTER_MAX.as_secs()));
            let mut entries = cache.entries.lock().expect("app cache poisoned");
            let previous = entries.remove(&key);
            let entry = match result {
                Ok(manifest) => CacheEntry {
                    manifest: Some(manifest),
                    fetched_at_unix_ms: unix_ms(),
                    last_good_at: Some(now),
                    next_refresh: now + REFRESH_INTERVAL + jitter,
                    error: None,
                },
                Err(error) => {
                    let keep = previous.as_ref().is_some_and(|entry| {
                        entry
                            .last_good_at
                            .is_some_and(|at| now.saturating_duration_since(at) <= LAST_GOOD_TTL)
                    });
                    CacheEntry {
                        manifest: if keep {
                            previous.as_ref().and_then(|entry| entry.manifest.clone())
                        } else {
                            None
                        },
                        fetched_at_unix_ms: previous
                            .as_ref()
                            .map_or(0, |entry| entry.fetched_at_unix_ms),
                        last_good_at: previous.as_ref().and_then(|entry| entry.last_good_at),
                        next_refresh: now + REFRESH_INTERVAL + jitter,
                        error: Some(error),
                    }
                }
            };
            entries.insert(key, entry);
        }
    }
}

pub(crate) fn start_refresher(directory_cache: Arc<DirectoryCache>, directory_file: PathBuf) {
    std::thread::Builder::new()
        .name("mini-app-manifests".to_string())
        .spawn(move || {
            let refresher = Refresher::production();
            loop {
                refresher.refresh(&directory_cache.read(&directory_file), global_cache());
                std::thread::sleep(DIRECTORY_POLL);
            }
        })
        .expect("spawn mini-app manifest refresher");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{Ipv4Addr, TcpListener};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn test_service(addr: SocketAddr) -> DirectoryService {
        DirectoryService {
            name: "keyed".to_string(),
            ip: addr.ip().to_string(),
            port: addr.port(),
            protocol: "http".to_string(),
            txt: BTreeMap::from([
                ("app".to_string(), "v1".to_string()),
                ("path".to_string(), "/app".to_string()),
            ]),
        }
    }

    fn directory_json(services: &[DirectoryService]) -> String {
        let services: Vec<_> = services
            .iter()
            .map(|service| {
                serde_json::json!({
                    "name": service.name,
                    "ip": service.ip,
                    "port": service.port,
                    "protocol": service.protocol,
                    "txt": service.txt,
                })
            })
            .collect();
        serde_json::json!({ "version": 1, "services": services }).to_string()
    }

    fn serve(
        responses: Vec<Vec<u8>>,
        delay: Duration,
    ) -> (
        SocketAddr,
        Arc<AtomicUsize>,
        Arc<Mutex<Vec<String>>>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let thread_count = Arc::clone(&count);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let handle = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut responses = responses.into_iter();
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        thread_count.fetch_add(1, Ordering::SeqCst);
                        let mut request = [0_u8; 2048];
                        let read = stream.read(&mut request).unwrap_or(0);
                        thread_requests
                            .lock()
                            .unwrap()
                            .push(String::from_utf8_lossy(&request[..read]).into_owned());
                        std::thread::sleep(delay);
                        if let Some(response) = responses.next() {
                            let _ = stream.write_all(&response);
                        }
                        if responses.len() == 0 {
                            break;
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        (addr, count, requests, handle)
    }

    fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        response
    }

    fn chunked_response(body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        response.extend_from_slice(b"\r\n0\r\n\r\n");
        response
    }

    fn local_fetcher(timeout: Duration) -> ManifestFetcher {
        ManifestFetcher::for_tests(|ip| ip.is_loopback(), timeout)
    }

    fn serve_tls_once(response: Vec<u8>) -> (SocketAddr, std::thread::JoinHandle<()>) {
        const CERT: &str = "MIIDCzCCAfOgAwIBAgIUWBRQQI8Hm24ZdmLJY1xQDmtTWigwDQYJKoZIhvcNAQELBQAwFTETMBEGA1UEAwwKa2V5ZWQubWVzaDAeFw0yNjA5MTQwMjAxMzJaFw0yNjA5MTYwMjAxMzJaMBUxEzARBgNVBAMMCmtleWVkLm1lc2gwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC2m975OBPEyDZ1c5WRE+YCpn7RTo5pBh57N0nX2Ak9SzTEzpmJ0KkcjxvdaMHtLTU3oBXHhOl+mIRRHUCHjTw1bpGiFOlSuwsmQMf0Wf7t6vEgpp7p+eLpVajbfb1KIk4UtkvWJ+dc1SlLxDZVwbCSPhQOWAiedkmkqGsMcqUeEN5XrZ/iCkZF8yCThk1KFfdYsrKBdZSm1fbUm+X0fbOGYwa3zRCIY4R0FAf9rd9wzS0loNYqhTRJLlD8ZdhPUG773T+UHMolZM+On+8uPXP9VjQpkY3c7H9j3kiIJNhqaq3ysQpv2I3F8xy1uhQT4+MxBKxF1TEv3HRSLdbmMOGlAgMBAAGjUzBRMB0GA1UdDgQWBBSnC5rATzyqC+291qz9pfQh9V+iBTAfBgNVHSMEGDAWgBSnC5rATzyqC+291qz9pfQh9V+iBTAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUAA4IBAQAJtMIfS/CANWaUDK5oYkbTgGu8PzaYI8rtg+KT0h7T91XzHMj0rN8mx+wR61H0/zMaiYi6z+RlKAfyzbekbGgab8cdoSXEIdW7S8le+XPu4Y/qcLX1WqEeZQQ1crWhCwqiN//uEZR+kUDkf6Pj85pNWrXcsf484JZkFgQ21zL3XbtLKgiWVc3xFhJiwMmLwrUU1itOJDMqM2I6te3Ho5nMnX7gp3jMrNe4mwimkb07M9EYVwwZeH7rEKMq3HfsiwMJyYcnb30OiBWqwBB05I5D1ZzIqwJVFQoZJgoXzBE3r3ijIJ6r9D0VwN6NS3/Kix7PhoBipHiki9Nb+C3DF2i+";
        const KEY: &str = "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC2m975OBPEyDZ1c5WRE+YCpn7RTo5pBh57N0nX2Ak9SzTEzpmJ0KkcjxvdaMHtLTU3oBXHhOl+mIRRHUCHjTw1bpGiFOlSuwsmQMf0Wf7t6vEgpp7p+eLpVajbfb1KIk4UtkvWJ+dc1SlLxDZVwbCSPhQOWAiedkmkqGsMcqUeEN5XrZ/iCkZF8yCThk1KFfdYsrKBdZSm1fbUm+X0fbOGYwa3zRCIY4R0FAf9rd9wzS0loNYqhTRJLlD8ZdhPUG773T+UHMolZM+On+8uPXP9VjQpkY3c7H9j3kiIJNhqaq3ysQpv2I3F8xy1uhQT4+MxBKxF1TEv3HRSLdbmMOGlAgMBAAECggEAJkj9QWKKsH/qfQr38XQKzgTywzbVYDsOcnZnxrduTnnBTT7kjXKgNhLh/HRtbijDhs9LKarZ2ncnDKuVnyXTGP++zEzWk0gQYRuFYJtquZ5/ogEVAciolJOnRKdy44NYO/rxYOu6U+oED7TKUIX8m1ilE1HNMs3piDpgQTqbGDeFaQFHKNrJ2Gx9c3K86Sh0MyMCL0W9c+9D1bjHdHANO3C2GMNhnM5dMi3saxQLqOrckvsCLMIb1fzHPoUvLfUp1vAKRcAGfz1DXrNGcLJxQj8uH1b7g1S4M+y6pNWcdz7oQBSiumH/c6/0q+wyDlONsFuw4KB8HBfhmaVz2ar7EQKBgQDyY+SMk4zF9bQkezlfsqPauTTg+XxyHa5fbimFPygliNDNJKONHonPh3Y3Zg8S92Bm4arVP1gvXQLYahYRU5N/lWSaz0PTlxWAugTbtTNXV9zNU6yc6J7CunbVd+fXYg56UQuveeOtUamSwqKddliedKyuvO3I+A7nuJ5cx6US0QKBgQDA3K8FYI3paMEIwZAtE0lW85lZhWdIvtilKhQ2H0uju8aFxLZ3f/dgj36Zyur5MLq+Cutd6FpCWALGlIx5gbgkL4/n2bt/Y6TKmEiveCW6pqrzHZYAtLqgpF/462PUt0T2uJG+o2yxlWIL4x4iP1UULLUMvv0gfFRQ2AMMMR2OlQKBgQCZayK425dpoQgBY0FAUiimAz31+9OJw0GgQ3DiVsRJZZyLi9o9MwwVH+9yRxXZclxBIirnyK0/ZUasxhDrrJOaWGuSFQggP+urS5JRohI6AXHPQFvsAMykAjO/D6Ldz8HMJ8oWqjayeBK1wp38vnB+8uhtvUVgQ6njfxY1MWRJUQKBgBOoas6JgO2Bl+tkj2WIybjrK35McrKfgUWUfGrn1bXiteF8o3yatoRJHAZhAIJVzTMBuevgexK4FdBX49metz44+toO/2WEqo9b5ky8WlwkENim81svELa/Cmk81Pghlg3v0is0TSfsqgm8JJ4pBmsAA5RBA1wEUDPNUMI1X2XhAoGBAIv0+Z0YjLXeyus/8td3BV9Ppmukj95BfF5WNaGi3SUPUpOkvdd1fzAlcTwNPz++H7IaBIQOc5akz2KMnz1i6yPhVKjsz1Rt4y/48nyCFgs3+EiQWS6hRZCHinESwvz/yJBlxug26eDqhvagvWcmjFSKfhtXDyqVGKc97kbdZWTY";

        let cert = CertificateDer::from(
            base64::engine::general_purpose::STANDARD
                .decode(CERT)
                .unwrap(),
        );
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(
            base64::engine::general_purpose::STANDARD
                .decode(KEY)
                .unwrap(),
        );
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key.into())
            .unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
            let mut stream = rustls::StreamOwned::new(connection, stream);
            let mut request = [0_u8; 2048];
            stream.read(&mut request).unwrap();
            stream.write_all(&response).unwrap();
        });
        (addr, handle)
    }

    #[test]
    fn manifest_validation_matches_shared_fixtures() {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../hello-mesh-web/src/lib/miniapp/fixtures/manifests");
        let mut paths: Vec<_> = std::fs::read_dir(fixture_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            let fixture: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let actual = manifest_from_value(fixture["input"].clone())
                .map(|manifest| serde_json::to_value(manifest).unwrap())
                .unwrap_or(Value::Null);
            assert_eq!(actual, fixture["expect"], "fixture {}", path.display());
        }
    }

    #[test]
    fn direct_record_ip_fetch_needs_no_cors_and_inlines_icon() {
        let manifest = br#"{"v":1,"name":"Keyed","icon":"/icon.png","embed":"card"}"#;
        let (addr, count, requests, handle) = serve(
            vec![
                response("200 OK", "Content-Type: application/json\r\n", manifest),
                response("200 OK", "Content-Type: image/png\r\n", b"png"),
            ],
            Duration::ZERO,
        );
        let manifest = local_fetcher(Duration::from_secs(1))
            .fetch(&test_service(addr))
            .unwrap();
        handle.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let requests = requests.lock().unwrap();
        assert!(requests.iter().all(|request| {
            request
                .to_ascii_lowercase()
                .contains("host: keyed.mesh\r\n")
        }));
        assert_eq!(manifest.name.as_deref(), Some("Keyed"));
        assert_eq!(manifest.embed, EmbedMode::Card);
        assert_eq!(manifest.icon.as_deref(), Some("data:image/png;base64,cG5n"));
    }

    #[test]
    fn self_signed_https_app_is_decorated() {
        let body = br#"{"v":1,"name":"Secure Keyed","embed":"card"}"#;
        let (addr, handle) = serve_tls_once(response(
            "200 OK",
            "Content-Type: application/json\r\n",
            body,
        ));
        let mut service = test_service(addr);
        service.protocol = "https".to_string();
        let manifest = local_fetcher(Duration::from_secs(1))
            .fetch(&service)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(manifest.name.as_deref(), Some("Secure Keyed"));
    }

    #[test]
    fn out_of_range_ip_is_not_fetched() {
        let service = DirectoryService {
            ip: "203.0.113.9".to_string(),
            port: 80,
            ..test_service("127.0.0.1:80".parse().unwrap())
        };
        let refresher = Refresher {
            fetcher: ManifestFetcher::production(),
        };
        let cache = AppCache::default();
        refresher.refresh(&directory_json(&[service]), &cache);
        let response = cache.response(Instant::now());
        assert!(response.apps[0].manifest.is_none());
        assert!(
            response.apps[0]
                .error
                .as_deref()
                .unwrap()
                .contains("outside mesh ranges")
        );
    }

    #[test]
    fn redirect_is_not_followed() {
        let (addr, count, _, handle) = serve(
            vec![response(
                "302 Found",
                "Location: http://other.mesh/manifest\r\n",
                b"",
            )],
            Duration::ZERO,
        );
        assert!(
            local_fetcher(Duration::from_secs(1))
                .fetch(&test_service(addr))
                .is_err()
        );
        handle.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn body_over_cap_is_rejected_after_transfer_reading() {
        let body = vec![b' '; MAX_MANIFEST_BYTES + 1];
        let (addr, _, _, handle) = serve(vec![chunked_response(&body)], Duration::ZERO);
        let err = local_fetcher(Duration::from_secs(1))
            .fetch(&test_service(addr))
            .unwrap_err();
        handle.join().unwrap();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn slow_response_times_out() {
        let body = br#"{"v":1}"#;
        let (addr, _, _, handle) = serve(
            vec![response(
                "200 OK",
                "Content-Type: application/json\r\n",
                body,
            )],
            Duration::from_millis(150),
        );
        assert!(
            local_fetcher(Duration::from_millis(30))
                .fetch(&test_service(addr))
                .is_err()
        );
        handle.join().unwrap();
    }

    #[test]
    fn cache_prunes_removed_and_changed_records() {
        let body = br#"{"v":1,"name":"Keyed"}"#;
        let (first_addr, _, _, first_handle) = serve(
            vec![response(
                "200 OK",
                "Content-Type: application/json\r\n",
                body,
            )],
            Duration::ZERO,
        );
        let refresher = Refresher {
            fetcher: local_fetcher(Duration::from_secs(1)),
        };
        let cache = AppCache::default();
        let first = test_service(first_addr);
        refresher.refresh(&directory_json(std::slice::from_ref(&first)), &cache);
        first_handle.join().unwrap();
        assert_eq!(cache.response(Instant::now()).apps.len(), 1);

        let (second_addr, _, _, second_handle) = serve(
            vec![response(
                "200 OK",
                "Content-Type: application/json\r\n",
                body,
            )],
            Duration::ZERO,
        );
        let changed = test_service(second_addr);
        refresher.refresh(&directory_json(std::slice::from_ref(&changed)), &cache);
        second_handle.join().unwrap();
        let response = cache.response(Instant::now());
        assert_eq!(response.apps.len(), 1);
        assert_eq!(response.apps[0].port, second_addr.port());

        refresher.refresh(&directory_json(&[]), &cache);
        assert!(cache.response(Instant::now()).apps.is_empty());
    }

    #[test]
    fn api_snapshot_is_memory_only_and_fast() {
        let cache = AppCache::default();
        let started = Instant::now();
        assert_eq!(cache.response_json(), r#"{"version":1,"apps":[]}"#);
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn successful_entry_is_not_refetched_before_five_minutes() {
        let body = br#"{"v":1}"#;
        let (addr, count, _, handle) = serve(
            vec![response(
                "200 OK",
                "Content-Type: application/json\r\n",
                body,
            )],
            Duration::ZERO,
        );
        let service = test_service(addr);
        let directory = directory_json(std::slice::from_ref(&service));
        let cache = AppCache::default();
        let refresher = Refresher {
            fetcher: local_fetcher(Duration::from_secs(1)),
        };
        refresher.refresh(&directory, &cache);
        refresher.refresh(&directory, &cache);
        handle.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn last_good_expires_after_one_hour() {
        let cache = AppCache::default();
        let key = AppKey {
            name: "keyed".to_string(),
            protocol: "http".to_string(),
            ip: "10.42.7.20".to_string(),
            port: 3000,
        };
        let now = Instant::now();
        cache.entries.lock().unwrap().insert(
            key,
            CacheEntry {
                manifest: manifest_from_value(serde_json::json!({ "v": 1 })),
                fetched_at_unix_ms: 1,
                last_good_at: Some(now - LAST_GOOD_TTL - Duration::from_secs(1)),
                next_refresh: now,
                error: Some("unreachable".to_string()),
            },
        );
        let response = cache.response(now);
        assert!(response.apps[0].manifest.is_none());
        assert!(response.apps[0].stale);
    }

    #[test]
    fn directory_app_count_is_bounded() {
        let services: Vec<_> = (0..MAX_APPS + 10)
            .map(|index| DirectoryService {
                name: format!("app-{index}"),
                ip: "203.0.113.9".to_string(),
                port: 80,
                protocol: "http".to_string(),
                txt: BTreeMap::from([("app".to_string(), "v1".to_string())]),
            })
            .collect();
        let cache = AppCache::default();
        Refresher::production().refresh(&directory_json(&services), &cache);
        assert_eq!(cache.response(Instant::now()).apps.len(), MAX_APPS);
    }
}
