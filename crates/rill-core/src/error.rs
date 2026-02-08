//! Error types for the Rill protocol.
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TransactionError {
    #[error("unknown UTXO: {0}")] UnknownUtxo(String),
    #[error("insufficient funds: have {have}, need {need}")] InsufficientFunds { have: u64, need: u64 },
    #[error("invalid signature on input {index}")] InvalidSignature { index: usize },
    #[error("duplicate input: {0}")] DuplicateInput(String),
    #[error("oversized: {size} > {max}")] OversizedTransaction { size: usize, max: usize },
    #[error("empty inputs or outputs")] EmptyInputsOrOutputs,
    #[error("value overflow")] ValueOverflow,
    #[error("invalid coinbase: {0}")] InvalidCoinbase(String),
    #[error("serialization: {0}")] Serialization(String),
    #[error("immature coinbase UTXO at input {index}")] ImmatureCoinbase { index: usize },
    #[error("zero-value output at index {0}")] ZeroValueOutput(usize),
    #[error("null outpoint in non-coinbase input {0}")] NullOutpointInRegularTx(usize),
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    #[error("invalid PoW")] InvalidPoW,
    #[error("invalid prev hash")] InvalidPrevHash,
    #[error("timestamp too far: {0}")] TimestampTooFar(i64),
    #[error("timestamp not after parent")] TimestampNotAfterParent,
    #[error("invalid merkle root")] InvalidMerkleRoot,
    #[error("invalid reward: got {got}, expected {expected}")] InvalidReward { got: u64, expected: u64 },
    #[error("invalid decay summary: {0}")] InvalidDecaySummary(String),
    #[error("oversized: {size} > {max}")] OversizedBlock { size: usize, max: usize },
    #[error("no coinbase")] NoCoinbase,
    #[error("first transaction is not coinbase")] FirstTxNotCoinbase,
    #[error("multiple coinbase transactions")] MultipleCoinbase,
    #[error("duplicate txid: {0}")] DuplicateTxid(String),
    #[error("double spend across transactions: {0}")] DoubleSpend(String),
    #[error("invalid difficulty: got {got}, expected {expected}")] InvalidDifficulty { got: u64, expected: u64 },
    #[error("tx error in {index}: {source}")] TransactionError { index: usize, source: TransactionError },
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DecayError {
    #[error("cluster not found: {0}")] ClusterNotFound(String),
    #[error("zero circulating supply")] ZeroCirculatingSupply,
    #[error("arithmetic overflow")] ArithmeticOverflow,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    #[error("peer disconnected: {0}")] PeerDisconnected(String),
    #[error("message too large: {size}")] MessageTooLarge { size: usize },
    #[error("timeout")] Timeout,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    #[error("invalid public key bytes")] InvalidPublicKey,
    #[error("invalid signature bytes")] InvalidSignature,
    #[error("signature verification failed")] VerificationFailed,
    #[error("pubkey hash does not match expected")] PubkeyHashMismatch,
    #[error("input index out of bounds: {index} >= {len}")] InputIndexOutOfBounds { index: usize, len: usize },
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    #[error("invalid HRP")] InvalidHrp,
    #[error("invalid length")] InvalidLength,
    #[error("invalid checksum")] InvalidChecksum,
    #[error("invalid character: {0}")] InvalidCharacter(char),
    #[error("invalid version: {0}")] InvalidVersion(u8),
    #[error("invalid padding bits")] InvalidPadding,
    #[error("unknown network: {0}")] UnknownNetwork(String),
    #[error("missing separator")] MissingSeparator,
    #[error("mixed case")] MixedCase,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MempoolError {
    #[error("transaction already in pool: {0}")] AlreadyExists(String),
    #[error("conflicts with pool tx {existing_txid} on outpoint {outpoint}")] Conflict { new_txid: String, existing_txid: String, outpoint: String },
    #[error("pool full")] PoolFull,
    #[error("internal: {0}")] Internal(String),
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ChainStateError {
    #[error("empty chain: no blocks connected")] EmptyChain,
    #[error("block not found: {0}")] BlockNotFound(String),
    #[error("undo data missing for block: {0}")] UndoDataMissing(String),
    #[error("height mismatch: expected {expected}, got {got}")] HeightMismatch { expected: u64, got: u64 },
    #[error("duplicate block: {0}")] DuplicateBlock(String),
}

#[derive(Error, Debug)]
pub enum RillError {
    #[error(transparent)] Transaction(#[from] TransactionError),
    #[error(transparent)] Block(#[from] BlockError),
    #[error(transparent)] Decay(#[from] DecayError),
    #[error(transparent)] Network(#[from] NetworkError),
    #[error(transparent)] Crypto(#[from] CryptoError),
    #[error(transparent)] Address(#[from] AddressError),
    #[error(transparent)] Mempool(#[from] MempoolError),
    #[error(transparent)] ChainState(#[from] ChainStateError),
    #[error("storage: {0}")] Storage(String),
}
