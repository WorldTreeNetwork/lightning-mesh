use mjolnir_apply::{
    Engine, NodeLock, OpenWrtAdapter, Plan, TxnPaths, active_plan, write_result_from_receipt,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn value(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("missing {name}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).ok_or_else(|| {
        "usage: mjolnir-txn <apply|recover|restore|verify> --lock-fd 9 ...".to_owned()
    })?;
    let lock_fd: i32 = value(&args, "--lock-fd")?
        .parse()
        .map_err(|_| "--lock-fd must be an integer".to_owned())?;
    let lock = NodeLock::inherit(lock_fd).map_err(|error| error.to_string())?;
    let paths = TxnPaths::production();
    let engine = Engine::new(paths.clone());

    match command {
        "apply" => {
            let plan_path = PathBuf::from(value(&args, "--plan")?);
            let source = PathBuf::from(value(&args, "--source-dir")?);
            let result = PathBuf::from(value(&args, "--result")?);
            let bytes = fs::read(&plan_path)
                .map_err(|error| format!("read {}: {error}", plan_path.display()))?;
            let plan: Plan = serde_json::from_slice(&bytes)
                .map_err(|error| format!("parse {}: {error}", plan_path.display()))?;
            let mut adapter = OpenWrtAdapter::for_apply(paths, source, plan.clone());
            match engine.apply_locked(&lock, &plan, &mut adapter) {
                Ok(receipt) => {
                    write_result_from_receipt(&result, &receipt).map_err(|error| error.to_string())
                }
                Err(error) => {
                    // Recovery-required is returned as an error, but its durable
                    // receipt is still the sole authority for the poller result.
                    if let Some(receipt) = engine
                        .receipt(&plan.transaction_id)
                        .map_err(|receipt_error| receipt_error.to_string())?
                    {
                        write_result_from_receipt(&result, &receipt)
                            .map_err(|result_error| result_error.to_string())?;
                    }
                    Err(error.to_string())
                }
            }
        }
        "recover" => {
            let plan = active_plan(&paths).map_err(|error| error.to_string())?;
            let mut adapter = match plan {
                Some(plan) => OpenWrtAdapter::for_apply(paths, PathBuf::new(), plan),
                None => OpenWrtAdapter::new(paths),
            };
            engine
                .recover_with_lock(&lock, &mut adapter)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        "restore" => engine
            .restore_before_network(&lock)
            .map(|_| ())
            .map_err(|error| error.to_string()),
        "verify" => {
            let plan = active_plan(&paths).map_err(|error| error.to_string())?;
            let mut adapter = match plan {
                Some(plan) => OpenWrtAdapter::for_apply(paths, PathBuf::new(), plan),
                None => OpenWrtAdapter::new(paths),
            };
            engine
                .verify_after_services(&lock, &mut adapter)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        _ => Err(format!("unknown command: {command}")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("mjolnir-txn: {error}");
        std::process::exit(1);
    }
}
