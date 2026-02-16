//! Wire message types for the Rill P2P protocol.
//!
//! All messages are serialized as MAGIC_BYTES prefix + bincode payload.
//! Never JSON for consensus-adjacent data.

use rill_core::constants::{MAGIC_BYTES, MAX_BLOCK_SIZE, MAX_LOCATOR_SIZE};
use rill_core::error::NetworkError;
use rill_core::types::{Block, BlockHeader, Hash256, Transaction};

/// Gossipsub topic for block propagation.
pub const BLOCKS_TOPIC: &str = "/rill/blocks/1";

/// Gossipsub topic for transaction propagation.
pub const TXS_TOPIC: &str = "/rill/txs/1";

/// Maximum wire message size (block size + overhead for framing).
pub const MAX_MESSAGE_SIZE: usize = MAX_BLOCK_SIZE + 1024;

/// A network message sent between Rill peers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub enum NetworkMessage {
    /// A new validated block to propagate.
    NewBlock(Block),
    /// A new validated transaction to propagate.
    NewTransaction(Transaction),
    /// Request a specific block by hash.
    GetBlock(Hash256),
    /// Request block headers starting from locator hashes.
    GetHeaders(Vec<Hash256>),
}

impl NetworkMessage {
    /// Validate message constraints before encoding or after decoding.
    ///
    /// VULN-10 fix: Enforces MAX_LOCATOR_SIZE for GetHeaders messages.
    pub fn validate(&self) -> Result<(), NetworkError> {
        if let NetworkMessage::GetHeaders(locator) = self {
            if locator.len() > MAX_LOCATOR_SIZE {
                return Err(NetworkError::LocatorTooLarge {
                    size: locator.len(),
                    max: MAX_LOCATOR_SIZE,
                });
            }
        }
        Ok(())
    }

    /// Encode this message as MAGIC_BYTES + bincode payload.
    ///
    /// Returns an error if the encoded size exceeds [`MAX_MESSAGE_SIZE`]
    /// or message validation fails.
    pub fn encode(&self) -> Result<Vec<u8>, NetworkError> {
        self.validate()?;
        let payload = bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| NetworkError::PeerDisconnected(format!("encode error: {e}")))?;
        let total_size = MAGIC_BYTES.len() + payload.len();
        if total_size > MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooLarge { size: total_size });
        }
        let mut buf = Vec::with_capacity(total_size);
        buf.extend_from_slice(&MAGIC_BYTES);
        buf.extend_from_slice(&payload);
        Ok(buf)
    }

    /// Decode a message from MAGIC_BYTES + bincode payload.
    ///
    /// Returns `None` if the magic bytes don't match, the message is too large,
    /// deserialization fails, or message validation fails.
    pub fn decode(data: &[u8]) -> Option<Self> {
        // VULN-04 fix: Check size limit before attempting deserialization
        if data.len() > MAX_MESSAGE_SIZE {
            return None;
        }
        if data.len() < MAGIC_BYTES.len() {
            return None;
        }
        if data[..MAGIC_BYTES.len()] != MAGIC_BYTES {
            return None;
        }
        let payload = &data[MAGIC_BYTES.len()..];
        let (msg, _): (Self, usize) =
            bincode::decode_from_slice(payload, bincode::config::standard()).ok()?;
        // VULN-10 fix: Validate message after decoding
        msg.validate().ok()?;
        Some(msg)
    }

    /// Returns the gossipsub topic this message should be published to.
    pub fn topic(&self) -> &'static str {
        match self {
            NetworkMessage::NewBlock(_) | NetworkMessage::GetBlock(_) => BLOCKS_TOPIC,
            NetworkMessage::NewTransaction(_) | NetworkMessage::GetHeaders(_) => TXS_TOPIC,
        }
    }
}

/// Point-to-point request types for the Rill req-resp protocol.
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum RillRequest {
    /// Request a block by its hash.
    GetBlock(Hash256),
    /// Request headers from locator hashes.
    GetHeaders(Vec<Hash256>),
}

/// Point-to-point response types for the Rill req-resp protocol.
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum RillResponse {
    /// A block response.
    Block(Option<Block>),
    /// Headers response.
    Headers(Vec<BlockHeader>),
}

/// Maximum request size (hash + locator overhead).
pub const MAX_REQUEST_SIZE: usize = 32 * 64 + 128; // 64 locator hashes + overhead

/// Maximum response size (up to one full block or many headers).
pub const MAX_RESPONSE_SIZE: usize = MAX_BLOCK_SIZE + 1024;

/// Protocol name for request-response.
pub const REQ_RESP_PROTOCOL: &str = "/rill/req-resp/1";

/// Codec for Rill request-response protocol.
/// Uses 4-byte length prefix + bincode payload.
#[derive(Debug, Clone, Default)]
pub struct RillCodec;

#[async_trait::async_trait]
impl libp2p::request_response::Codec for RillCodec {
    type Protocol = libp2p::StreamProtocol;
    type Request = RillRequest;
    type Response = RillResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        use libp2p::futures::AsyncReadExt;
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_REQUEST_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request too large",
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        let (request, _): (RillRequest, _) = bincode::decode_from_slice(&buf, bincode::config::standard())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        use libp2p::futures::AsyncReadExt;
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_RESPONSE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "response too large",
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        let (response, _): (RillResponse, _) = bincode::decode_from_slice(&buf, bincode::config::standard())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        use libp2p::futures::AsyncWriteExt;
        let buf = bincode::encode_to_vec(&req, bincode::config::standard())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if buf.len() > MAX_REQUEST_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request too large",
            ));
        }
        let len = (buf.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&buf).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        use libp2p::futures::AsyncWriteExt;
        let buf = bincode::encode_to_vec(&resp, bincode::config::standard())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        if buf.len() > MAX_RESPONSE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "response too large",
            ));
        }
        let len = (buf.len() as u32).to_be_bytes();
        io.write_all(&len).await?;
        io.write_all(&buf).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rill_core::types::{BlockHeader, TxInput, TxOutput};

    fn sample_block() -> Block {
        Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256::ZERO,
                merkle_root: Hash256::ZERO,
                timestamp: 1_700_000_000,
                difficulty_target: u64::MAX,
                nonce: 42,
            },
            transactions: vec![Transaction {
                version: 1,
                inputs: vec![TxInput {
                    previous_output: rill_core::types::OutPoint::null(),
                    signature: vec![],
                    public_key: vec![],
                }],
                outputs: vec![TxOutput {
                    value: 50 * rill_core::constants::COIN,
                    pubkey_hash: Hash256::ZERO,
                }],
                lock_time: 0,
            }],
        }
    }

    fn sample_tx() -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![TxInput {
                previous_output: rill_core::types::OutPoint {
                    txid: Hash256([0x11; 32]),
                    index: 0,
                },
                signature: vec![0u8; 64],
                public_key: vec![0u8; 32],
            }],
            outputs: vec![TxOutput {
                value: 100,
                pubkey_hash: Hash256([0xAA; 32]),
            }],
            lock_time: 0,
        }
    }

    #[test]
    fn round_trip_new_block() {
        let msg = NetworkMessage::NewBlock(sample_block());
        let encoded = msg.encode().unwrap();
        let decoded = NetworkMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, NetworkMessage::NewBlock(_)));
    }

    #[test]
    fn round_trip_new_transaction() {
        let msg = NetworkMessage::NewTransaction(sample_tx());
        let encoded = msg.encode().unwrap();
        let decoded = NetworkMessage::decode(&encoded).unwrap();
        assert!(matches!(decoded, NetworkMessage::NewTransaction(_)));
    }

    #[test]
    fn round_trip_get_block() {
        let hash = Hash256([0xBB; 32]);
        let msg = NetworkMessage::GetBlock(hash);
        let encoded = msg.encode().unwrap();
        let decoded = NetworkMessage::decode(&encoded).unwrap();
        match decoded {
            NetworkMessage::GetBlock(h) => assert_eq!(h, hash),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn round_trip_get_headers() {
        let locator = vec![Hash256([1; 32]), Hash256([2; 32])];
        let msg = NetworkMessage::GetHeaders(locator.clone());
        let encoded = msg.encode().unwrap();
        let decoded = NetworkMessage::decode(&encoded).unwrap();
        match decoded {
            NetworkMessage::GetHeaders(l) => assert_eq!(l, locator),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn wrong_magic_rejected() {
        let msg = NetworkMessage::GetBlock(Hash256::ZERO);
        let mut encoded = msg.encode().unwrap();
        // Corrupt the magic bytes
        encoded[0] = 0x00;
        assert!(NetworkMessage::decode(&encoded).is_none());
    }

    #[test]
    fn too_short_rejected() {
        assert!(NetworkMessage::decode(&[0x52, 0x49]).is_none());
    }

    #[test]
    fn empty_data_rejected() {
        assert!(NetworkMessage::decode(&[]).is_none());
    }

    #[test]
    fn topic_routing_blocks() {
        let msg = NetworkMessage::NewBlock(sample_block());
        assert_eq!(msg.topic(), BLOCKS_TOPIC);
    }

    #[test]
    fn topic_routing_get_block() {
        let msg = NetworkMessage::GetBlock(Hash256::ZERO);
        assert_eq!(msg.topic(), BLOCKS_TOPIC);
    }

    #[test]
    fn topic_routing_transactions() {
        let msg = NetworkMessage::NewTransaction(sample_tx());
        assert_eq!(msg.topic(), TXS_TOPIC);
    }

    #[test]
    fn topic_routing_get_headers() {
        let msg = NetworkMessage::GetHeaders(vec![]);
        assert_eq!(msg.topic(), TXS_TOPIC);
    }

    #[test]
    fn encoded_starts_with_magic() {
        let msg = NetworkMessage::GetBlock(Hash256::ZERO);
        let encoded = msg.encode().unwrap();
        assert_eq!(&encoded[..4], &MAGIC_BYTES);
    }

    #[test]
    fn constants_are_correct() {
        assert_eq!(BLOCKS_TOPIC, "/rill/blocks/1");
        assert_eq!(TXS_TOPIC, "/rill/txs/1");
        assert_eq!(MAX_MESSAGE_SIZE, MAX_BLOCK_SIZE + 1024);
    }

    #[test]
    fn request_encode_decode() {
        let req = RillRequest::GetBlock(Hash256([0xBB; 32]));
        let encoded = bincode::encode_to_vec(&req, bincode::config::standard()).unwrap();
        let (decoded, _): (RillRequest, _) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        match decoded {
            RillRequest::GetBlock(h) => assert_eq!(h, Hash256([0xBB; 32])),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn response_encode_decode() {
        let resp = RillResponse::Block(None);
        let encoded = bincode::encode_to_vec(&resp, bincode::config::standard()).unwrap();
        let (decoded, _): (RillResponse, _) =
            bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        match decoded {
            RillResponse::Block(None) => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn codec_max_size_constants() {
        assert!(MAX_REQUEST_SIZE > 32); // at least one hash
        assert!(MAX_RESPONSE_SIZE >= MAX_MESSAGE_SIZE); // can fit a full block
    }
}
