use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Microdegrees: 1e-7 deg ≈ 1.1 cm at the equator. The CRDT wire unit for lat/lon.
pub const MICRODEGREES: i32 = 10_000_000;

/// Last-known WGS84 stamp for one mesh node (bead mjolnir-mesh-6hn.2).
///
/// Fixed-point microdegrees (`i32`, 1e-7 deg) so the entry can derive [`Eq`]
/// — raw `f64` is not `Eq` and is a bad equality/idempotence substrate for a
/// CRDT. Altitude is millimetres above the WGS84 ellipsoid when present.
///
/// # Authority
///
/// **The subject may differ from the stamper.** A phone on a node's client
/// SSID stamps a router; `node_id` is the subject, `stamper` is attribution
/// only. Do **not** drop a stamp because `node_id != stamper` — that check
/// belongs to [`NodeNameEntry`](crate::crdt::node_name::NodeNameEntry) (a node
/// only announces its own name) and would silently kill every third-party
/// coordinate.
///
/// Gossip verifies **no signatures**. A stamp is an untrusted-but-attributed
/// claim. The living `mesh-coverage` spec's "hello verifies the signature,
/// spools the claim" sentence is hello's job (`add-compass-node-mark`); meshd
/// does not re-verify anything on ingest or re-announce.
///
/// `stamped_at_unix` is the author's claim (typically a phone wall clock).
/// meshd does not rewrite it and does not clamp a stamp from the future.
///
/// There is no unstamp/tombstone: a wrong coordinate is only correctable by a
/// newer one.
///
/// Stored at `/coordinates/{node_id}` in the CRDT coordinate book. The book
/// is keyed by subject `node_id`, not `backhaul_addr` (addresses are derived
/// from the id; consumers join directory → radio on `backhaul_addr`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinateStamp {
    /// 64-hex iroh node id of the subject (the node being stamped).
    /// May differ from [`Self::stamper`].
    pub node_id: String,
    /// WGS84 latitude in microdegrees (1e-7 deg).
    pub lat_e7: i32,
    /// WGS84 longitude in microdegrees (1e-7 deg).
    pub lon_e7: i32,
    /// Altitude in millimetres above the WGS84 ellipsoid, when known.
    pub alt_mm: Option<i32>,
    /// Author's Unix time in seconds. Not rewritten by meshd; not clamped.
    pub stamped_at_unix: u64,
    /// Attribution only (phone identikey, later a compass). Not an authority
    /// check and not a signature.
    pub stamper: String,
}

/// Mesh-wide last-known coordinate book: subject `node_id` → most recent stamp.
///
/// The key must equal `entry.node_id`. Unlike the node-name book, the stamper
/// is *not* required to equal the subject — a phone stamps a router.
pub type CoordinateBook = BTreeMap<String, CoordinateStamp>;

/// Directory JSON projection of a last-known stamp (degrees / metres).
///
/// Flattened onto `DirectoryNode` / `DirectoryNeighbor`. Serialization omits
/// every field when unset (`skip_serializing_if`) so an unmarked node has no
/// `lat`/`lon` keys — never `(0,0)` and never JSON `null`.
///
/// Converted from [`CoordinateStamp`]'s microdegrees / millimetres at
/// projection time; the CRDT entry itself stays integer.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CoordinateProjection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamped_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamper: Option<String>,
}

impl CoordinateStamp {
    /// Directory JSON shape: degrees and metres, for flatten onto a node/neighbor.
    pub fn projection(&self) -> CoordinateProjection {
        CoordinateProjection {
            lat: Some(f64::from(self.lat_e7) / f64::from(MICRODEGREES)),
            lon: Some(f64::from(self.lon_e7) / f64::from(MICRODEGREES)),
            alt: self.alt_mm.map(|mm| f64::from(mm) / 1000.0),
            stamped_at: Some(self.stamped_at_unix),
            stamper: Some(self.stamper.clone()),
        }
    }
}

/// Convert WGS84 degrees to microdegrees (`i32` e7).
///
/// Rounds half away from zero (Rust `f64::round`). `None` if non-finite or
/// the rounded value does not fit in `i32`.
pub fn degrees_to_e7(deg: f64) -> Option<i32> {
    if !deg.is_finite() {
        return None;
    }
    let v = (deg * f64::from(MICRODEGREES)).round();
    if v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
        None
    } else {
        Some(v as i32)
    }
}

/// Convert metres to millimetres. Same rounding / overflow rules as
/// [`degrees_to_e7`].
pub fn metres_to_mm(metres: f64) -> Option<i32> {
    if !metres.is_finite() {
        return None;
    }
    let v = (metres * 1000.0).round();
    if v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
        None
    } else {
        Some(v as i32)
    }
}

/// Look up `node_id` in `book` and project it. Unmarked → all fields `None`
/// (JSON omits the keys).
pub fn project_coordinate(book: &CoordinateBook, node_id: &str) -> CoordinateProjection {
    book.get(node_id)
        .map(CoordinateStamp::projection)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::merge::{MergeResult, apply_coordinate, merge_coordinate};

    fn stamp(
        node_id: &str,
        lat_e7: i32,
        lon_e7: i32,
        stamped_at_unix: u64,
        stamper: &str,
    ) -> CoordinateStamp {
        CoordinateStamp {
            node_id: node_id.to_string(),
            lat_e7,
            lon_e7,
            alt_mm: None,
            stamped_at_unix,
            stamper: stamper.to_string(),
        }
    }

    #[test]
    fn coordinate_postcard_roundtrip() {
        let original = CoordinateStamp {
            node_id: "abcd1234".repeat(8),
            lat_e7: 377_749_000,
            lon_e7: -1_224_194_000,
            alt_mm: Some(12_345),
            stamped_at_unix: 1_700_000_000,
            stamper: "phone-ada".to_string(),
        };
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: CoordinateStamp = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn coordinate_postcard_roundtrip_book() {
        let mut book = CoordinateBook::new();
        book.insert(
            "router-a".to_string(),
            stamp(
                "router-a",
                370_000_000,
                -122_000_000,
                1_700_000_000,
                "phone",
            ),
        );
        let bytes = postcard::to_allocvec(&book).unwrap();
        let decoded: CoordinateBook = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(book, decoded);
    }

    #[test]
    fn coordinate_unmarked_serialize_omits_lat_lon_keys() {
        // Headline unmarked invariant: a None projection must omit the keys,
        // not serialize them as null. Asserted on JSON (a Rust None still
        // satisfies a struct assertion while serializing `null`).
        let unmarked = CoordinateProjection::default();
        let json = serde_json::to_string(&unmarked).expect("serializes");
        assert_eq!(json, "{}");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("lat").is_none(), "lat omitted: {json}");
        assert!(value.get("lon").is_none(), "lon omitted: {json}");
        assert!(value.get("alt").is_none(), "alt omitted: {json}");
        assert!(
            value.get("stamped_at").is_none(),
            "stamped_at omitted: {json}"
        );
        assert!(value.get("stamper").is_none(), "stamper omitted: {json}");
    }

    #[test]
    fn coordinate_marked_serialize_includes_lat_lon_stamper() {
        let marked = stamp(
            "wr3000s-a",
            370_000_000,
            -1_220_000_000,
            1_700_000_042,
            "phone-ada",
        )
        .projection();
        let json = serde_json::to_string(&marked).expect("serializes");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["lat"], 37.0);
        assert_eq!(value["lon"], -122.0);
        assert!(
            value.get("alt").is_none(),
            "no alt_mm → alt omitted: {json}"
        );
        assert_eq!(value["stamped_at"], 1_700_000_042);
        assert_eq!(value["stamper"], "phone-ada");
    }

    #[test]
    fn coordinate_marked_serialize_includes_alt_metres() {
        let mut s = stamp("m3000", 0, 0, 1, "phone");
        s.alt_mm = Some(1_500);
        let value = serde_json::to_value(s.projection()).unwrap();
        assert_eq!(value["alt"], 1.5);
    }

    #[test]
    fn coordinate_project_unmarked_node_is_default() {
        let book = CoordinateBook::new();
        let proj = project_coordinate(&book, "ghost");
        assert_eq!(proj, CoordinateProjection::default());
        let json = serde_json::to_string(&proj).unwrap();
        assert!(!json.contains("lat") && !json.contains("lon"), "{json}");
    }

    #[test]
    fn coordinate_third_party_stamper_accepted() {
        // A phone stamps a router: subject != stamper. Merge AND apply must
        // accept that — the node_name integrity arm (subject == announcer)
        // must not be copied here.
        let incoming = stamp(
            "router-a",
            370_000_000,
            -122_000_000,
            1_700_000_000,
            "phone-ada",
        );
        assert_ne!(incoming.node_id, incoming.stamper);
        assert!(matches!(
            merge_coordinate(None, &incoming),
            MergeResult::Inserted
        ));

        let mut book = CoordinateBook::new();
        assert!(matches!(
            apply_coordinate(&mut book, &incoming),
            MergeResult::Inserted
        ));
        assert_eq!(book.get("router-a"), Some(&incoming));
    }

    #[test]
    fn coordinate_eq_is_total() {
        let a = stamp("n", 1, 2, 3, "s");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn degrees_to_e7_round_trip_exact() {
        assert_eq!(degrees_to_e7(37.0), Some(370_000_000));
        assert_eq!(degrees_to_e7(-122.0), Some(-1_220_000_000));
        assert_eq!(degrees_to_e7(0.0), Some(0));
    }

    #[test]
    fn degrees_to_e7_rejects_non_finite() {
        assert_eq!(degrees_to_e7(f64::NAN), None);
        assert_eq!(degrees_to_e7(f64::INFINITY), None);
        assert_eq!(metres_to_mm(f64::NEG_INFINITY), None);
    }

    #[test]
    fn metres_to_mm_converts() {
        assert_eq!(metres_to_mm(1.5), Some(1_500));
        assert_eq!(metres_to_mm(-0.001), Some(-1));
    }
}
