//! Mock Account definition and builder related methods.

use eth_types::{geth_types::Account, Address, Bytes, Word};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
/// Mock structure which represents an Account and can be used for tests.
pub struct MockAccount {
    /// Address
    pub address: Address,
    /// nonce
    pub nonce: Word,
    /// Balance
    pub balance: Word,
    /// EVM Code
    pub code: Bytes,
    /// Storage
    pub storage: HashMap<Word, Word>,
}

impl From<MockAccount> for Account {
    fn from(mock: MockAccount) -> Self {
        Account {
            address: mock.address,
            nonce: mock.nonce,
            balance: mock.balance,
            code: mock.code,
            storage: mock.storage,
        }
    }
}

impl MockAccount {
    /// Set address field for the MockAccount.
    pub fn address(&mut self, address: Address) -> &mut Self {
        self.address = address;
        self
    }

    /// Set nonce field for the MockAccount.
    pub fn nonce(&mut self, nonce: Word) -> &mut Self {
        self.nonce = nonce;
        self
    }

    /// Set balance field for the MockAccount.
    pub fn balance(&mut self, balance: Word) -> &mut Self {
        self.balance = balance;
        self
    }

    /// Set code field for the MockAccount.
    pub fn code(&mut self, code: Bytes) -> &mut Self {
        self.code = code;
        self
    }

    /// Add storage field for the MockAccount by passing an iterator over the
    /// [`Word`].
    pub fn storage<I: Iterator<Item = (Word, Word)>>(&mut self, storage: I) -> &mut Self {
        storage.for_each(|pair| {
            self.storage
                .insert(pair.0, pair.1)
                .expect("Error including storage pair");
        });
        self
    }

    /// Finalizes the current MockAccount under construction returning a new
    /// instance to it.
    pub fn build(&mut self) -> Self {
        self.to_owned()
    }
}