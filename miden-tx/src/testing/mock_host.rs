// MOCK HOST
// ================================================================================================

use alloc::{rc::Rc, string::ToString, sync::Arc};

use miden_lib::transaction::TransactionEvent;
use miden_objects::{
    accounts::{AccountHeader, AccountVaultDelta},
    Digest,
};
use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceInputs, AdviceProvider, AdviceSource, ContextId,
    ExecutionError, Host, HostResponse, MastForest, MastForestStore, MemAdviceProvider,
    ProcessState,
};

use crate::{host::AccountProcedureIndexMap, TransactionMastStore};
// MOCK HOST
// ================================================================================================

/// This is very similar to the TransactionHost in miden-tx. The differences include:
/// - We do not track account delta here.
/// - There is special handling of EMPTY_DIGEST in account procedure index map.
/// - This host uses `MemAdviceProvider` which is instantiated from the passed in advice inputs.
pub struct MockHost {
    adv_provider: MemAdviceProvider,
    acct_procedure_index_map: AccountProcedureIndexMap,
    mast_store: Rc<TransactionMastStore>,
}

impl MockHost {
    /// Returns a new [MockHost] instance with the provided [AdviceInputs].
    pub fn new(
        account: AccountHeader,
        advice_inputs: AdviceInputs,
        mast_store: Rc<TransactionMastStore>,
    ) -> Self {
        let adv_provider: MemAdviceProvider = advice_inputs.into();
        let proc_index_map =
            AccountProcedureIndexMap::new([account.code_commitment()], &adv_provider);
        Self {
            adv_provider,
            acct_procedure_index_map: proc_index_map.unwrap(),
            mast_store,
        }
    }

    /// Consumes `self` and returns the advice provider and account vault delta.
    pub fn into_parts(self) -> (MemAdviceProvider, AccountVaultDelta) {
        (self.adv_provider, AccountVaultDelta::default())
    }

    // EVENT HANDLERS
    // --------------------------------------------------------------------------------------------

    fn on_push_account_procedure_index<S: ProcessState>(
        &mut self,
        process: &S,
    ) -> Result<(), ExecutionError> {
        let proc_idx = self
            .acct_procedure_index_map
            .get_proc_index(process)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;
        self.adv_provider.push_stack(AdviceSource::Value(proc_idx.into()))?;
        Ok(())
    }
}

impl Host for MockHost {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.get_advice(process, &extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.set_advice(process, &injector)
    }

    fn get_mast_forest(&self, node_digest: &Digest) -> Option<Arc<MastForest>> {
        self.mast_store.get(node_digest)
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        let event = TransactionEvent::try_from(event_id)
            .map_err(|err| ExecutionError::EventError(err.to_string()))?;

        if process.ctx() != ContextId::root() {
            return Err(ExecutionError::EventError(format!(
                "{event} event can only be emitted from the root context"
            )));
        }

        match event {
            TransactionEvent::AccountPushProcedureIndex => {
                self.on_push_account_procedure_index(process)
            },
            _ => Ok(()),
        }?;

        Ok(HostResponse::None)
    }
}
