use crate::block::{BlockDate, Header, Proof};
use crate::key::{deserialize_public_key, serialize_public_key};
use crate::{
    leadership::{Error, ErrorKind, Verification},
    ledger::Ledger,
};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::bech32::{Bech32, Error as Bech32Error};
use chain_crypto::{Ed25519Extended, PublicKey, SecretKey};
use std::sync::Arc;

/// cryptographic signature algorithm used for the BFT leadership
/// protocol.
#[allow(non_camel_case_types)]
pub type SIGNING_ALGORITHM = Ed25519Extended;

/// BFT Leader signing key
pub type SigningKey = SecretKey<SIGNING_ALGORITHM>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeaderId(pub(crate) PublicKey<SIGNING_ALGORITHM>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BftRoundRobinIndex(u64);

/// The BFT Leader selection is based on a round robin of the expected leaders
#[derive(Debug)]
pub struct BftLeaderSelection {
    pub(crate) leaders: Arc<Vec<LeaderId>>,
}

impl BftLeaderSelection {
    /// Create a new BFT leadership
    pub fn new(ledger: &Ledger) -> Option<Self> {
        if ledger.settings.bft_leaders.len() == 0 {
            return None;
        }

        Some(BftLeaderSelection {
            leaders: Arc::clone(&ledger.settings.bft_leaders),
        })
    }

    #[inline]
    pub fn number_of_leaders(&self) -> usize {
        self.leaders.len()
    }

    #[inline]
    fn offset(&self, block_number: u64) -> BftRoundRobinIndex {
        let max = self.number_of_leaders() as u64;
        BftRoundRobinIndex((block_number % max) as u64)
    }

    pub(crate) fn verify(&self, block_header: &Header) -> Verification {
        match &block_header.proof() {
            Proof::Bft(bft_proof) => match self.get_leader_at(*block_header.block_date()) {
                Ok(leader_at) => {
                    if bft_proof.leader_id != leader_at {
                        Verification::Failure(Error::new(ErrorKind::InvalidLeader))
                    } else {
                        Verification::Success
                    }
                }
                Err(error) => Verification::Failure(error),
            },
            _ => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
        }
    }

    #[inline]
    pub(crate) fn get_leader_at(&self, date: BlockDate) -> Result<LeaderId, Error> {
        let BftRoundRobinIndex(ofs) = self.offset(date.slot_id as u64);
        Ok(self.leaders[ofs as usize].clone())
    }
}

impl LeaderId {
    pub fn as_public_key(&self) -> &PublicKey<SIGNING_ALGORITHM> {
        &self.0
    }
}

impl property::Serialize for LeaderId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl Readable for LeaderId {
    fn read<'a>(reader: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        deserialize_public_key(reader).map(LeaderId)
    }
}

impl AsRef<[u8]> for LeaderId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl From<PublicKey<SIGNING_ALGORITHM>> for LeaderId {
    fn from(v: PublicKey<SIGNING_ALGORITHM>) -> Self {
        LeaderId(v)
    }
}

impl Bech32 for LeaderId {
    const BECH32_HRP: &'static str = PublicKey::<SIGNING_ALGORITHM>::BECH32_HRP;

    fn try_from_bech32_str(s: &str) -> Result<Self, Bech32Error> {
        PublicKey::<SIGNING_ALGORITHM>::try_from_bech32_str(s).map(Self)
    }

    fn to_bech32_str(&self) -> String {
        self.0.to_bech32_str()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for LeaderId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            LeaderId(SecretKey::generate(&mut rng).to_public())
        }
    }
}
