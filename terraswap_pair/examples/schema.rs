use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use terraswap::pair::{
    Cw20HookMsg, ExecuteMsg, FeatureToggle, InstantiateMsg, MigrateMsg, PoolFee, PoolResponse,
    ProtocolFeesResponse, QueryMsg, ReverseSimulationResponse, SimulationResponse,
};
use terraswap_pair::state::{Config, ConfigResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema(&schema_for!(Cw20HookMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(FeatureToggle), &out_dir);
    export_schema(&schema_for!(PoolFee), &out_dir);
    export_schema(&schema_for!(PoolResponse), &out_dir);
    export_schema(&schema_for!(SimulationResponse), &out_dir);
    export_schema(&schema_for!(ProtocolFeesResponse), &out_dir);
    export_schema(&schema_for!(ReverseSimulationResponse), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema_with_title(&schema_for!(ConfigResponse), &out_dir, "ConfigResponse");
}
