//! Optional HTTPS origin. HTTP remains available even without TLS material.

use std::{error::Error, fs, io};

use tiny_http::{Request, Response, Server, SslConfig};

use crate::config::Config;

type TlsError = Box<dyn Error + Send + Sync>;

pub struct Origin {
    name: String,
    port: u16,
}

impl Origin {
    fn new(name: &str, port: u16) -> Result<Self, TlsError> {
        let name = name.to_ascii_lowercase();
        let labels: Vec<_> = name.split('.').collect();
        let owner = labels[0].strip_prefix("n-").unwrap_or("");
        if name.len() > 253
            || labels.len() < 3
            || owner.len() != 16
            || !owner
                .bytes()
                .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
            || labels.iter().any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--tls-name must be one n-<80-bit label> DNS origin",
            )
            .into());
        }
        Ok(Self { name, port })
    }

    fn allows(&self, headers: &[tiny_http::Header]) -> bool {
        let mut hosts = headers.iter().filter(|h| h.field.equiv("Host"));
        let Some(host) = hosts.next() else {
            return false;
        };
        hosts.next().is_none()
            && (host.value.as_str().eq_ignore_ascii_case(&self.name)
                || host
                    .value
                    .as_str()
                    .eq_ignore_ascii_case(&format!("{}:{}", self.name, self.port)))
    }

    pub fn check_request(&self, request: Request) -> Option<Request> {
        if self.allows(request.headers()) {
            return Some(request);
        }
        if let Err(err) =
            request.respond(Response::from_string("Misdirected Request\n").with_status_code(421))
        {
            tracing::warn!(%err, "failed to reject foreign TLS Host");
        }
        None
    }
}

/// Missing flags or either missing PEM file leave TLS off, without binding.
pub fn start(config: &Config) -> Result<Option<(Server, Origin)>, TlsError> {
    let (Some(cert), Some(key)) = (&config.tls_cert, &config.tls_key) else {
        return Ok(None);
    };
    if !cert.is_file() || !key.is_file() {
        return Ok(None);
    }
    let name = config
        .tls_name
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "TLS requires --tls-name"))?;
    let mut origin = Origin::new(name, 443)?;
    let bind = match &config.tls_bind {
        Some(bind) => bind.clone(),
        None => {
            let (host, _) = config.bind.rsplit_once(':').ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "--bind must include a port")
            })?;
            format!("{host}:443")
        }
    };
    let server = Server::https(
        &bind,
        SslConfig {
            certificate: fs::read(cert)?,
            private_key: fs::read(key)?,
        },
    )?;
    origin.port = server
        .server_addr()
        .to_ip()
        .expect("TCP TLS listener")
        .port();
    tracing::info!(%bind, name = %origin.name, "mjolnir-hello TLS listening");
    Ok(Some((server, origin)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tiny_http::Header;

    const NAME: &str = "n-abcdefghijklmnop.home.mesh.worldtree.network";

    #[test]
    fn tls_origin_optional_flags_and_default_paths() {
        let config = Config::parse_from(["hello"]);
        assert_eq!(config.bind, "127.0.0.1:8080");
        assert!(start(&config).unwrap().is_none());
        let config = Config::parse_from(["hello", "--tls-cert", "--tls-key"]);
        assert_eq!(
            config.tls_cert.unwrap().to_str(),
            Some("/etc/mjolnir/hello.crt")
        );
        assert_eq!(
            config.tls_key.unwrap().to_str(),
            Some("/etc/mjolnir/hello.key")
        );
    }

    #[test]
    fn tls_origin_missing_files_leave_tls_off() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("hello.crt");
        let key = dir.path().join("hello.key");
        let config = Config::parse_from([
            "hello",
            "--tls-cert",
            cert.to_str().unwrap(),
            "--tls-key",
            key.to_str().unwrap(),
            "--tls-name",
            NAME,
            "--tls-bind",
            "invalid-bind",
        ]);
        assert!(start(&config).unwrap().is_none());
        fs::write(&cert, "not a certificate").unwrap();
        assert!(start(&config).unwrap().is_none());
        fs::remove_file(&cert).unwrap();
        fs::write(&key, "not a key").unwrap();
        assert!(start(&config).unwrap().is_none());
        let mut config = config;
        config.tls_key = None;
        assert!(start(&config).unwrap().is_none());
    }

    #[test]
    fn tls_origin_rejects_foreign_missing_duplicate_and_wrong_port_hosts() {
        let origin = Origin::new(NAME, 443).unwrap();
        let host = |name: &str| Header::from_bytes("Host", name).unwrap();
        assert!(origin.allows(&[host(NAME)]));
        assert!(origin.allows(&[host(&format!("{NAME}:443"))]));
        assert!(origin.allows(&[host(&NAME.to_ascii_uppercase())]));
        for foreign in [
            "hello.mesh",
            "a-abcdefghijklmnop.home.mesh.worldtree.network",
            "evil.example",
            "",
            &format!("{NAME}.evil.example"),
            &format!("{NAME}:80"),
        ] {
            assert!(!origin.allows(&[host(foreign)]), "{foreign}");
        }
        assert!(!origin.allows(&[]));
        assert!(!origin.allows(&[host(NAME), host(NAME)]));
        let foreign = tiny_http::TestRequest::new()
            .with_https()
            .with_header(host("foreign.example"));
        assert!(origin.check_request(foreign.into()).is_none());
        let own = tiny_http::TestRequest::new()
            .with_https()
            .with_header(host(NAME));
        assert!(origin.check_request(own.into()).is_some());
    }

    #[test]
    fn tls_origin_requires_a_router_name() {
        for name in [
            "hello.mesh",
            "*.home.mesh.worldtree.network",
            "a-abcdefghijklmnop.home.mesh.worldtree.network",
            "n-short.home.example",
            "n-0123456789abcdef.home.example",
        ] {
            assert!(Origin::new(name, 443).is_err(), "{name}");
        }
    }

    #[test]
    fn tls_origin_init_guards_files_and_keeps_both_paths() {
        let script = include_str!("../../../deploy/openwrt/files/etc/init.d/mjolnir-hello");
        assert!(script.contains("'/etc/mjolnir/hello.crt'"));
        assert!(script.contains("'/etc/mjolnir/hello.key'"));
        assert!(script.contains("[ -f \"$tls_cert\" ] && [ -f \"$tls_key\" ]"));
        assert!(
            script.contains(
                "--tls-cert \"$tls_cert\" --tls-key \"$tls_key\" --tls-name \"$tls_name\""
            )
        );
        assert!(script.contains("for keep_path in \"$tls_cert\" \"$tls_key\""));
        assert!(script.contains("grep -qxF \"$keep_path\" /etc/sysupgrade.conf"));
        assert!(script.contains(">> /etc/sysupgrade.conf"));
        assert!(script.contains("procd_append_param command --bind \"$bind\""));
    }
}
