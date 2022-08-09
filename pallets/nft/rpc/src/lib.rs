use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject, INTERNAL_ERROR_CODE},
};
pub use parami_nft_rpc_runtime_api::{ApiResult, NftRuntimeApi};
use parami_primitives::{BalanceWrapper, DecentralizedId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
use std::sync::Arc;

#[rpc(client, server)]
pub trait NftApi<BlockHash, NftId, DecentralizedId, Balance>
where
    Balance: MaybeDisplay + MaybeFromStr,
{
    /// TODO(ironman_ch): add more comment here
    #[method(name = "nft_get_claim_info")]
    fn get_claim_info(
        &self,
        nft_id: NftId,
        claimer: DecentralizedId,
        at: Option<BlockHash>,
    ) -> RpcResult<(
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
    )>;
}

pub struct NftRpcHandler<C, Block, NftId, DecentralizedId, Balance> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<(Block, NftId, DecentralizedId, Balance)>,
}

impl<C, Block, NftId, DecentralizedId, Balance>
    NftRpcHandler<C, Block, NftId, DecentralizedId, Balance>
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, NftId, DecentralizedId, Balance>
    NftApiServer<<Block as BlockT>::Hash, NftId, DecentralizedId, Balance>
    for NftRpcHandler<C, Block, NftId, DecentralizedId, Balance>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: NftRuntimeApi<Block, NftId, DecentralizedId, Balance>,
    NftId: Codec + Send + Sync + 'static,
    DecentralizedId: Codec + Send + Sync + 'static,
    Balance: Codec + MaybeDisplay + MaybeFromStr + Send + Sync + 'static,
{
    fn get_claim_info(
        &self,
        nft_id: NftId,
        claimer: DecentralizedId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
    )> {
        let api = self.client.runtime_api();
        let at: BlockId<Block> = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let res = api.get_claim_info(&at, nft_id, claimer).map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to get claim info on nft.",
                Some(format!("{:?}", e)),
            )))
        })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to get claim info on nft.",
                Some(format!("{:?}", e)),
            )))
        })
    }
}
