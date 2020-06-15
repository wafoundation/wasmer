use crate::utils::get_store;
use anyhow::Result;

use wasmer::*;

#[test]
fn native_function_works_for_wasm() -> Result<()> {
    let store = get_store();
    let wat = r#"(module
        (func $multiply (import "env" "multiply") (param i32 i32) (result i32))
        (func (export "add") (param i32 i32) (result i32)
           (i32.add (local.get 0)
                    (local.get 1)))
        (func (export "double_then_add") (param i32 i32) (result i32)
           (i32.add (call $multiply (local.get 0) (i32.const 2))
                    (call $multiply (local.get 1) (i32.const 2))))
)"#;
    let module = Module::new(&store, wat).unwrap();

    let import_object = imports! {
        "env" => {
            "multiply" => Function::new(&store, |a: i32, b: i32| a * b),
        },
    };

    let instance = Instance::new(&module, &import_object)?;

    let f: NativeFunc<(i32, i32), i32> = instance.exports.get_native_function("add")?;
    let result = f.call(4, 6)?;
    assert_eq!(result, 10);

    let dyn_f: &Function = instance.exports.get("double_then_add")?;
    let dyn_result = dyn_f.call(&[Val::I32(4), Val::I32(6)])?;
    assert_eq!(dyn_result[0], Val::I32(20));

    let f: NativeFunc<(i32, i32), i32> = dyn_f.native().unwrap();

    let result = f.call(4, 6)?;
    assert_eq!(result, 20);
    Ok(())
}