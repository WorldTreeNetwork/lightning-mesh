// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Byte-stream capability handshake, yielding only a discovery locator.
//!
//! On [`CAPABILITY_ALPN`], the scanner sends exactly 32 raw nonce bytes. The
//! node replies with a four-byte big-endian envelope length followed by the
//! UTF-8 envelope from `sign_handshake` (no extra newline). One exchange uses
//! one stream; neither side needs to wait for EOF. This module opens no socket
//! and does not register the ALPN, grant membership, or lift quarantine.

use std::io::{self, Read, Write};

use ed25519_dalek::SigningKey;
use thiserror::Error;

use crate::capability_beacon::{
    BeaconError, BeaconLocator, CapabilityDescriptor, HandshakeChallenge, sign_handshake,
    verify_handshake,
};

pub const CAPABILITY_ALPN: &[u8] = b"mjolnir/capability-beacon/1";

/// Maximum UTF-8 envelope length, excluding the four-byte length prefix.
pub const MAX_HANDSHAKE_ENVELOPE_BYTES: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum CapabilityTransportError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Beacon(#[from] BeaconError),
}

/// Send and flush the scanner nonce, retaining its single-use verification
/// state. The caller MUST generate a fresh, unpredictable nonce for every new
/// challenge, including after a failed exchange or restart. Never recreate a
/// challenge with a previously used nonce.
pub fn write_challenge<W: Write>(
    writer: &mut W,
    locator: BeaconLocator,
    fresh_nonce: [u8; 32],
) -> Result<HandshakeChallenge, CapabilityTransportError> {
    writer.write_all(&fresh_nonce)?;
    writer.flush()?;
    Ok(HandshakeChallenge::new(locator, fresh_nonce))
}

/// Read the scanner's nonce and send exactly one signed, framed response.
/// The caller owns the stream's lifetime and read/write timeouts.
pub fn respond_to_challenge<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    descriptor: &CapabilityDescriptor,
    node_key: &SigningKey,
) -> Result<(), CapabilityTransportError> {
    let mut nonce = [0; 32];
    reader.read_exact(&mut nonce)?;
    writer.write_all(&handshake_response_frame(descriptor, &nonce, node_key)?)?;
    writer.flush()?;
    Ok(())
}

/// Build the same bounded response frame as [`respond_to_challenge`] for
/// transports that own their asynchronous stream I/O. This proves possession
/// of the node key only; it does not grant membership.
pub fn handshake_response_frame(
    descriptor: &CapabilityDescriptor,
    nonce: &[u8; 32],
    node_key: &SigningKey,
) -> Result<Vec<u8>, CapabilityTransportError> {
    let envelope = sign_handshake(descriptor, nonce, node_key)?;
    validate_envelope_length(envelope.len())?;
    let mut frame = Vec::with_capacity(4 + envelope.len());
    frame.extend_from_slice(&(envelope.len() as u32).to_be_bytes());
    frame.extend_from_slice(envelope.as_bytes());
    Ok(frame)
}

/// Read one bounded frame and delegate signature, locator, nonce, and replay
/// checks to `verify_handshake`. Success consumes the challenge and returns
/// only a locator, never an authorization or membership grant.
pub fn read_response<R: Read>(
    reader: &mut R,
    challenge: &mut HandshakeChallenge,
) -> Result<BeaconLocator, CapabilityTransportError> {
    let mut length = [0; 4];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    validate_envelope_length(length)?;
    let mut envelope = vec![0; length];
    reader.read_exact(&mut envelope)?;
    let envelope = std::str::from_utf8(&envelope)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(verify_handshake(envelope, challenge)?.locator().clone())
}

fn validate_envelope_length(length: usize) -> io::Result<()> {
    if length == 0 || length > MAX_HANDSHAKE_ENVELOPE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capability handshake envelope length is out of bounds",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_beacon::{BootstrapCoordinates, ChannelPlan, sign_beacon, verify_beacon};
    use std::io::Cursor;

    fn node() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            protocol_generation: 1,
            node_id: node()
                .verifying_key()
                .to_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            channel_plan: vec![ChannelPlan {
                frequency_mhz: 5180,
                width_mhz: 80,
            }],
            bootstrap: BootstrapCoordinates {
                direct_addrs: vec!["10.254.0.1:7000".parse().unwrap()],
                relay_url: None,
            },
        }
    }

    fn locator() -> BeaconLocator {
        verify_beacon(&sign_beacon(&descriptor(), &node()).unwrap()).unwrap()
    }

    fn response(nonce: [u8; 32]) -> Vec<u8> {
        let mut response = Vec::new();
        respond_to_challenge(&mut nonce.as_slice(), &mut response, &descriptor(), &node()).unwrap();
        response
    }

    fn assert_io_error(error: CapabilityTransportError, kind: io::ErrorKind) {
        match error {
            CapabilityTransportError::Io(error) => assert_eq!(error.kind(), kind),
            other => panic!("expected I/O error, got {other:?}"),
        }
    }

    #[test]
    fn pinned_alpn_and_in_memory_exchange_return_only_a_locator() {
        assert_eq!(CAPABILITY_ALPN, b"mjolnir/capability-beacon/1");
        let nonce = [0xa5; 32];
        let mut request = Vec::new();
        let mut challenge = write_challenge(&mut request, locator(), nonce).unwrap();
        assert_eq!(request, nonce);
        let mut response = Vec::new();
        respond_to_challenge(
            &mut request.as_slice(),
            &mut response,
            &descriptor(),
            &node(),
        )
        .unwrap();
        let envelope = sign_handshake(&descriptor(), &nonce, &node()).unwrap();
        assert_eq!(&response[..4], &(envelope.len() as u32).to_be_bytes());
        assert_eq!(&response[4..], envelope.as_bytes());
        // A following byte is left unread: the frame does not depend on EOF.
        response.push(0xff);
        let mut reader = Cursor::new(response);
        let found: BeaconLocator = read_response(&mut reader, &mut challenge).unwrap();
        assert_eq!(found, locator());
        assert_eq!(reader.position(), (4 + envelope.len()) as u64);
    }

    #[test]
    fn response_frame_matches_stream_response() {
        let nonce = [0xa5; 32];
        let frame = handshake_response_frame(&descriptor(), &nonce, &node()).unwrap();
        assert_eq!(frame, response(nonce));
        let envelope = sign_handshake(&descriptor(), &nonce, &node()).unwrap();
        assert_eq!(&frame[..4], &(envelope.len() as u32).to_be_bytes());
        assert_eq!(&frame[4..], envelope.as_bytes());
    }

    #[test]
    fn replay_and_response_to_an_old_nonce_fail() {
        let frame = handshake_response_frame(&descriptor(), &[1; 32], &node()).unwrap();
        let mut challenge = write_challenge(&mut Vec::new(), locator(), [1; 32]).unwrap();
        read_response(&mut frame.as_slice(), &mut challenge).unwrap();
        assert!(matches!(
            read_response(&mut frame.as_slice(), &mut challenge),
            Err(CapabilityTransportError::Beacon(BeaconError::ReplayedNonce))
        ));
        let mut fresh = write_challenge(&mut Vec::new(), locator(), [2; 32]).unwrap();
        assert!(matches!(
            read_response(&mut frame.as_slice(), &mut fresh),
            Err(CapabilityTransportError::Beacon(BeaconError::NonceMismatch))
        ));
        assert_eq!(
            read_response(&mut response([2; 32]).as_slice(), &mut fresh).unwrap(),
            locator()
        );
    }

    #[test]
    fn tampered_signature_and_changed_locator_fail() {
        let mut challenge = write_challenge(&mut Vec::new(), locator(), [3; 32]).unwrap();
        let mut tampered = response([3; 32]);
        let last = tampered.last_mut().unwrap();
        *last = if *last == b'0' { b'1' } else { b'0' };
        assert!(matches!(
            read_response(&mut tampered.as_slice(), &mut challenge),
            Err(CapabilityTransportError::Beacon(
                BeaconError::ForgedSignature
            ))
        ));
        let mut other = descriptor();
        other.channel_plan[0].frequency_mhz = 5200;
        let mut changed = Vec::new();
        respond_to_challenge(&mut [3; 32].as_slice(), &mut changed, &other, &node()).unwrap();
        assert!(matches!(
            read_response(&mut changed.as_slice(), &mut challenge),
            Err(CapabilityTransportError::Beacon(
                BeaconError::LocatorMismatch
            ))
        ));
        assert!(read_response(&mut response([3; 32]).as_slice(), &mut challenge).is_ok());
    }

    #[test]
    fn incomplete_nonce_and_wrong_signing_key_write_no_response() {
        let mut output = Vec::new();
        assert_io_error(
            respond_to_challenge(&mut [0; 31].as_slice(), &mut output, &descriptor(), &node())
                .unwrap_err(),
            io::ErrorKind::UnexpectedEof,
        );
        assert!(output.is_empty());
        assert!(matches!(
            respond_to_challenge(
                &mut [0; 32].as_slice(),
                &mut output,
                &descriptor(),
                &SigningKey::from_bytes(&[8; 32]),
            ),
            Err(CapabilityTransportError::Beacon(
                BeaconError::NodeIdMismatch
            ))
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn invalid_lengths_are_rejected_before_reading_a_body() {
        for length in [0, MAX_HANDSHAKE_ENVELOPE_BYTES as u32 + 1, u32::MAX] {
            let mut challenge = write_challenge(&mut Vec::new(), locator(), [4; 32]).unwrap();
            assert_io_error(
                read_response(&mut length.to_be_bytes().as_slice(), &mut challenge).unwrap_err(),
                io::ErrorKind::InvalidData,
            );
        }
        let mut oversized = descriptor();
        oversized.bootstrap.relay_url = Some("x".repeat(MAX_HANDSHAKE_ENVELOPE_BYTES));
        let mut output = Vec::new();
        assert_io_error(
            respond_to_challenge(&mut [4; 32].as_slice(), &mut output, &oversized, &node())
                .unwrap_err(),
            io::ErrorKind::InvalidData,
        );
        assert!(output.is_empty());
    }

    #[test]
    fn truncated_and_invalid_utf8_responses_fail() {
        let valid = response([5; 32]);
        for frame in [&valid[..3], &valid[..valid.len() - 1]] {
            let mut challenge = write_challenge(&mut Vec::new(), locator(), [5; 32]).unwrap();
            assert_io_error(
                read_response(&mut &*frame, &mut challenge).unwrap_err(),
                io::ErrorKind::UnexpectedEof,
            );
        }
        let mut challenge = write_challenge(&mut Vec::new(), locator(), [5; 32]).unwrap();
        assert_io_error(
            read_response(&mut [0, 0, 0, 1, 0xff].as_slice(), &mut challenge).unwrap_err(),
            io::ErrorKind::InvalidData,
        );
    }

    // Force short reads/writes and require flushing without opening any socket.
    struct Fragmented<T>(T, bool);

    impl<T: Read> Read for Fragmented<T> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let length = buf.len().min(1);
            self.0.read(&mut buf[..length])
        }
    }

    impl<T: Write> Write for Fragmented<T> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(&buf[..buf.len().min(1)])
        }

        fn flush(&mut self) -> io::Result<()> {
            self.1 = true;
            self.0.flush()
        }
    }

    #[test]
    fn fragmented_streams_are_read_fully_and_writes_are_flushed() {
        let mut request = Fragmented(Vec::new(), false);
        let mut challenge = write_challenge(&mut request, locator(), [6; 32]).unwrap();
        assert!(request.1);
        assert_eq!(request.0, [6; 32]);
        let mut response = Fragmented(Vec::new(), false);
        respond_to_challenge(
            &mut Fragmented(request.0.as_slice(), false),
            &mut response,
            &descriptor(),
            &node(),
        )
        .unwrap();
        assert!(response.1);
        assert_eq!(
            read_response(
                &mut Fragmented(response.0.as_slice(), false),
                &mut challenge
            )
            .unwrap(),
            locator()
        );
    }

    #[test]
    fn write_errors_propagate() {
        let mut full = [0; 0];
        assert_io_error(
            write_challenge(&mut full.as_mut_slice(), locator(), [7; 32]).unwrap_err(),
            io::ErrorKind::WriteZero,
        );
        assert_io_error(
            respond_to_challenge(
                &mut [7; 32].as_slice(),
                &mut full.as_mut_slice(),
                &descriptor(),
                &node(),
            )
            .unwrap_err(),
            io::ErrorKind::WriteZero,
        );
    }
}
