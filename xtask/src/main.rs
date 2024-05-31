#![cfg(not(tarpaulin_include))]

use std::{env, error::Error};

/// Traverses through the workspace and generates schemas
/// Expects to be ran at the workspace.

pub mod tasks {
    use std::{
        collections::HashMap,
        env,
        fs::{create_dir_all, write},
        path::{Path, PathBuf},
        process::Command,
    };

    use cosmwasm_schema::{generate_api, remove_schemas};

    use serde::Deserialize;
    use white_whale_std::{
        pool_network::{frontend_helper, incentive, incentive_factory},
        vault_network::{vault, vault_factory, vault_router},
        *,
    };

    fn project_root() -> PathBuf {
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(1)
            .unwrap()
            .to_path_buf()
    }

    #[derive(Deserialize)]
    struct CargoPackage {
        name: String,
        version: String,
        manifest_path: String,
    }

    #[derive(Deserialize)]
    struct CargoMetadataOutput {
        packages: Vec<CargoPackage>,
    }

    pub fn generate_schemas() {
        macro_rules! generate_schema {
            ($contract:expr, $api_ref: expr) => {
                (
                    $contract.to_string(),
                    generate_api! {
                        instantiate: $api_ref::InstantiateMsg,
                        query: $api_ref::QueryMsg,
                        execute: $api_ref::ExecuteMsg,
                        migrate: $api_ref::MigrateMsg,
                    },
                )
            };
        }

        let mut schemas = HashMap::from([
            generate_schema!("bonding-manager", bonding_manager),
            generate_schema!("epoch-manager", epoch_manager::epoch_manager),
            generate_schema!("fee_collector", fee_collector),
            generate_schema!("fee_distributor", fee_distributor),
            generate_schema!("pool-manager", pool_manager),
            generate_schema!("incentive-manager", incentive_manager),
            generate_schema!("frontend-helper", frontend_helper),
            generate_schema!("incentive", incentive),
            generate_schema!("incentive-factory", incentive_factory),
            generate_schema!("terraswap-factory", pool_network::factory),
            generate_schema!("terraswap-pair", pool_network::pair),
            generate_schema!("terraswap-router", pool_network::router),
            generate_schema!("terraswap-token", pool_network::token),
            generate_schema!("vault-manager", vault_manager),
            generate_schema!("vault", vault),
            generate_schema!("vault_factory", vault_factory),
            generate_schema!("vault_router", vault_router),
            generate_schema!("whale-lair", whale_lair),
        ]);

        let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let metadata = Command::new(cargo)
            .current_dir(project_root())
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .output()
            .expect("Failed to fetch workspace metadata");

        let metadata = serde_json::from_slice::<CargoMetadataOutput>(&metadata.stdout)
            .expect("Failed to parse `cargo metadata` output");

        let contracts = metadata
            .packages
            .into_iter()
            .filter(|member| member.manifest_path.contains("contracts"))
            .filter(|member| {
                member.name != "fee-distributor-mock"
                    && member.name != "stableswap-3pool"
                    && member.name != "stable-swap-sim"
                    && member.name != "stable-swap-sim1"
            });

        for contract in contracts {
            let contract_path = Path::new(&contract.manifest_path)
                .parent()
                .expect("Failed to get parent of contract manifest");

            // generate correct schema version
            let mut schema = schemas.remove(&contract.name).unwrap_or_else(|| {
                panic!(
                    "Missing contract {} defined in xtask generate schemas",
                    contract.name
                )
            });
            schema.contract_version = contract.version;
            schema.contract_name = contract.name.clone();

            let mut out_dir = contract_path.to_path_buf();
            out_dir.push("schema");
            create_dir_all(&out_dir).unwrap_or_else(|e| {
                panic!(
                    "Failed to create out dir for contract {}: {e}",
                    contract.name
                )
            });
            remove_schemas(&out_dir).unwrap_or_else(|e| {
                panic!(
                    "Failed to create out dir for contract {}: {e}",
                    contract.name
                )
            });

            let api = schema.render();
            let path = out_dir.join(format!("{}.json", contract.name));
            let json = api.to_string().unwrap_or_else(|e| {
                panic!(
                    "Failed to serialize JsonApi when generating schema for {}: {}",
                    contract.name, e
                )
            });
            write(&path, json + "\n").unwrap_or_else(|e| {
                panic!(
                    "Failed to write JsonApi when generating schema for {}: {}",
                    contract.name, e
                )
            });
            println!("Generated schemas for {}", contract.name);

            let raw_dir = out_dir.join("raw");
            create_dir_all(&raw_dir).unwrap_or_else(|e| {
                panic!(
                    "Failed to create raw schema directory for {}: {}",
                    contract.name, e
                )
            });
            for (filename, json) in api.to_schema_files().unwrap_or_else(|e| {
                panic!("Failed to get schema files for {}: {}", contract.name, e)
            }) {
                let path = raw_dir.join(&filename);

                write(&path, json + "\n").unwrap_or_else(|e| {
                    panic!("Failed to write raw schema file to {}: {}", filename, e)
                });
            }
        }
    }

    pub fn print_help() {
        println!(
            "
Usage: Run with `cargo xtask <task>`, eg. `cargo xtask generate_schemas`.

    Tasks:
        generate_schemas: Generate schemas for each contract.
"
        );
    }
}

// reference: https://github.com/helix-editor/helix/blob/master/xtask/src/main.rs
fn main() -> Result<(), Box<dyn Error>> {
    let task = env::args().nth(1);

    match task {
        None => tasks::print_help(),
        Some(t) => match t.as_str() {
            "generate_schemas" => tasks::generate_schemas(),
            invalid => return Err(format!("Invalid task name: {}", invalid).into()),
        },
    };

    Ok(())
}
