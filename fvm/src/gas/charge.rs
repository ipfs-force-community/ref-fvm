// Copyright 2019-2022 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::gas::Milligas;

/// Single gas charge in the VM. Contains information about what gas was for, as well
/// as the amount of gas needed for computation and storage respectively.
pub struct GasCharge<'a> {
    pub name: &'a str,
    /// Compute costs in milligas.
    pub compute_gas: Milligas,
    /// Storage costs in milligas.
    pub storage_gas: Milligas,
}

impl<'a> GasCharge<'a> {
    pub fn new(name: &'a str, compute_gas: Milligas, storage_gas: Milligas) -> Self {
        Self {
            name,
            compute_gas,
            storage_gas,
        }
    }

    /// Calculates total gas charge (in milligas) by summing compute and
    /// storage gas associated with this charge.
    pub fn total(&self) -> Milligas {
        self.compute_gas + self.storage_gas
    }
}
