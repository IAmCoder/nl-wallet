use dashmap::DashMap;

use nl_wallet_mdoc::{
    holder::{Credential, CredentialCopies, CredentialStorage},
    DocType, Error,
};

/// An implementation of [`CredentialStorage`] using maps, structured as follows::
/// - mdocs with different doctypes, through the map over `DocType`,
/// - multiple mdocs having the same doctype but distinct attributes, through the map over `Vec<u8>` which is computed
///   with [`Credential::hash()`] (see its rustdoc for details),
/// - multiple mdocs having the same doctype and the same attributes, through the `CredentialCopies` data structure.
#[derive(Debug, Clone, Default)]
pub struct Credentials(pub(crate) DashMap<DocType, DashMap<Vec<u8>, CredentialCopies>>);

impl<const N: usize> TryFrom<[Credential; N]> for Credentials {
    type Error = Error;

    fn try_from(value: [Credential; N]) -> Result<Self, Self::Error> {
        let creds = Credentials(DashMap::new());
        creds.add(value.into_iter())?;
        Ok(creds)
    }
}

impl Credentials {
    pub fn new() -> Credentials {
        Credentials(DashMap::new())
    }
}

impl CredentialStorage for Credentials {
    fn add(&self, creds: impl Iterator<Item = Credential>) -> Result<(), Error> {
        for cred in creds.into_iter() {
            self.0
                .entry(cred.doc_type.clone())
                .or_insert(DashMap::new())
                .entry(cred.hash()?)
                .or_insert(CredentialCopies::new())
                .creds
                .push(cred);
        }

        Ok(())
    }

    fn get(&self, doctype: &DocType) -> Option<Vec<CredentialCopies>> {
        self.0
            .get(doctype)
            .map(|v| v.value().iter().map(|entry| entry.value().clone()).collect())
    }
}
