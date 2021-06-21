//! Util
use crate::{
    derive::{HostFuncType, ReturnValue, Value},
    Error,
};
use ceres_std::vec;
use core::mem;
use wasmtime::{
    Caller,
    Config,
    Engine,
    Func,
    FuncType,
    Store,
    Trap,
    Val, // WasmBacktraceDetails,
};

/// Create store with DWARF enabled
///
/// NOTE: The Debug info with native trace has some problem in
/// aarch64-apple-darwin, not enable for default for now.
pub fn store_with_dwarf() -> Result<Store, Error> {
    Ok(Store::new(
        &Engine::new(
		&Config::new()
			// .debug_info(true)
			// .wasm_backtrace_details(WasmBacktraceDetails::Enable),
	)
        .map_err(|_| Error::CreateWasmtimeConfigFailed)?,
    ))
}

/// Wrap host function into `Func`
pub fn wrap_fn<T>(store: &Store, state: usize, f: usize, sig: FuncType) -> Func {
    let func = move |_: Caller<'_>, args: &[Val], results: &mut [Val]| {
        let mut inner_args = vec![];
        for arg in args {
            if let Some(arg) = from_val(arg.clone()) {
                inner_args.push(arg);
            } else {
                return Err(Trap::new("Could not wrap host function"));
            }
        }

        // HACK the LIFETIME
        //
        // # Safety
        //
        // Work for one call.
        let state: &mut T = unsafe { mem::transmute(state) };
        let func: HostFuncType<T> = unsafe { mem::transmute(f) };
        match func(state, &inner_args) {
            Ok(ret) => {
                if let Some(ret) = from_ret_val(ret) {
                    results[0] = ret;
                }
                Ok(())
            }
            Err(e) => Err(Trap::new(format!("{:?}", e))),
        }
    };
    Func::new(store, sig, func)
}

pub fn from_val(v: Val) -> Option<Value> {
    match v {
        Val::F32(v) => Some(Value::F32(v)),
        Val::I32(v) => Some(Value::I32(v)),
        Val::F64(v) => Some(Value::F64(v)),
        Val::I64(v) => Some(Value::I64(v)),
        _ => None,
    }
}

pub fn to_val(v: Value) -> Val {
    match v {
        Value::F32(v) => Val::F32(v),
        Value::F64(v) => Val::F64(v),
        Value::I32(v) => Val::I32(v),
        Value::I64(v) => Val::I64(v),
    }
}

pub fn to_ret_val(v: Val) -> Result<ReturnValue, Error> {
    from_val(v)
        .map(ReturnValue::Value)
        .ok_or(Error::UnExpectedReturnValue)
}

fn from_ret_val(v: ReturnValue) -> Option<Val> {
    match v {
        ReturnValue::Value(v) => match v {
            Value::I64(v) => Some(Val::I64(v)),
            Value::F64(v) => Some(Val::F64(v)),
            Value::I32(v) => Some(Val::I32(v)),
            Value::F32(v) => Some(Val::F32(v)),
        },
        ReturnValue::Unit => None,
    }
}
