use ibc::core::ics24_host::identifier::ChainId;
use ibc_proto::cosmos::tx::v1beta1::{Fee, Tx};
use ibc_proto::google::protobuf::Any;
use tonic::codegen::http::Uri;
use tracing::{debug, error, span, warn, Level};

use crate::chain::cosmos::account::{AccountNumber, AccountSequence};
use crate::chain::cosmos::encode::sign_tx;
use crate::chain::cosmos::gas::{gas_amount_to_fees, PrettyFee};
use crate::chain::cosmos::simulate::send_tx_simulate;
use crate::chain::cosmos::types::GasConfig;
use crate::config::types::Memo;
use crate::config::ChainConfig;
use crate::error::Error;
use crate::keyring::KeyEntry;

pub async fn estimate_tx_fees(
    config: &ChainConfig,
    grpc_address: &Uri,
    account_sequence: AccountSequence,
    account_number: AccountNumber,
    messages: Vec<Any>,
    key_entry: &KeyEntry,
    tx_memo: &Memo,
) -> Result<Fee, Error> {
    let gas_config = GasConfig::from_chain_config(config);

    debug!(
        "max fee, for use in tx simulation: {}",
        PrettyFee(&gas_config.max_fee)
    );

    let signed_tx = sign_tx(
        config,
        messages,
        account_sequence,
        key_entry,
        &gas_config.max_fee,
        tx_memo,
        account_number,
    )?;

    let tx = Tx {
        body: Some(signed_tx.body),
        auth_info: Some(signed_tx.auth_info),
        signatures: signed_tx.signatures,
    };

    let estimated_fee = estimate_fee_with_tx(&gas_config, &config.id, grpc_address, tx).await?;

    Ok(estimated_fee)
}

async fn estimate_fee_with_tx(
    gas_config: &GasConfig,
    chain_id: &ChainId,
    grpc_address: &Uri,
    tx: Tx,
) -> Result<Fee, Error> {
    let estimated_gas = estimate_gas_with_tx(gas_config, grpc_address, tx).await?;

    if estimated_gas > gas_config.max_gas {
        debug!(
            id = %chain_id, estimated = ?estimated_gas, max = ?gas_config.max_gas,
            "send_tx: estimated gas is higher than max gas"
        );

        return Err(Error::tx_simulate_gas_estimate_exceeded(
            chain_id.clone(),
            estimated_gas,
            gas_config.max_gas,
        ));
    }

    let adjusted_fee = gas_amount_to_fees(gas_config, estimated_gas);

    debug!(
        id = %chain_id,
        "send_tx: using {} gas, fee {}",
        estimated_gas,
        PrettyFee(&adjusted_fee)
    );

    Ok(adjusted_fee)
}

async fn estimate_gas_with_tx(
    gas_config: &GasConfig,
    grpc_address: &Uri,
    tx: Tx,
) -> Result<u64, Error> {
    let simulated_gas = send_tx_simulate(tx, grpc_address)
        .await
        .map(|sr| sr.gas_info);

    let _span = span!(Level::ERROR, "estimate_gas").entered();

    match simulated_gas {
        Ok(Some(gas_info)) => {
            debug!(
                "tx simulation successful, gas amount used: {:?}",
                gas_info.gas_used
            );

            Ok(gas_info.gas_used)
        }

        Ok(None) => {
            warn!(
                "tx simulation successful but no gas amount used was returned, falling back on default gas: {}",
                gas_config.default_gas
            );

            Ok(gas_config.default_gas)
        }

        // If there is a chance that the tx will be accepted once actually submitted, we fall
        // back on the max gas and will attempt to send it anyway.
        // See `can_recover_from_simulation_failure` for more info.
        Err(e) if can_recover_from_simulation_failure(&e) => {
            warn!(
                "failed to simulate tx, falling back on default gas because the error is potentially recoverable: {}",
                e.detail()
            );

            Ok(gas_config.default_gas)
        }

        Err(e) => {
            error!(
                "failed to simulate tx. propagating error to caller: {}",
                e.detail()
            );
            // Propagate the error, the retrying mechanism at caller may catch & retry.
            Err(e)
        }
    }
}

/// Determine whether the given error yielded by `tx_simulate`
/// can be recovered from by submitting the tx anyway.
fn can_recover_from_simulation_failure(e: &Error) -> bool {
    use crate::error::ErrorDetail::*;

    match e.detail() {
        GrpcStatus(detail) => detail.is_client_state_height_too_low(),
        _ => false,
    }
}
