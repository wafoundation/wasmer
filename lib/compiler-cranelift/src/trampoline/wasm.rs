//! A trampoline generator for calling Wasm functions easily.
//!
//! That way, you can start calling Wasm functions doing things like:
//! ```ignore
//! let my_func = instance.exports.get("func");
//! my_func.call([1, 2])
//! ```
use super::binemit::TrampolineRelocSink;
use crate::translator::signature_to_cranelift_ir;
use crate::{compiled_function_unwind_info, transform_jump_table};
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::print_errors::pretty_error;
use cranelift_codegen::Context;
use cranelift_codegen::{binemit, ir};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use wasm_common::FuncType;
use wasmer_compiler::FunctionAddressMap;
use wasmer_compiler::{CompileError, CompiledFunction};

/// Create a trampoline for invoking a WebAssembly function.
pub fn make_wasm_trampoline(
    isa: &dyn TargetIsa,
    fn_builder_ctx: &mut FunctionBuilderContext,
    func_type: &FuncType,
    value_size: usize,
) -> Result<CompiledFunction, CompileError> {
    let pointer_type = isa.pointer_type();
    let frontend_config = isa.frontend_config();
    let signature = signature_to_cranelift_ir(func_type, &frontend_config);
    let mut wrapper_sig = ir::Signature::new(frontend_config.default_call_conv);

    // Add the callee `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::special(
        pointer_type,
        ir::ArgumentPurpose::VMContext,
    ));

    // Add the caller `vmctx` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    // Add the `callee_address` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    // Add the `values_vec` parameter.
    wrapper_sig.params.push(ir::AbiParam::new(pointer_type));

    let mut context = Context::new();
    context.func = ir::Function::with_name_signature(ir::ExternalName::user(0, 0), wrapper_sig);
    context.func.collect_frame_layout_info();

    {
        let mut builder = FunctionBuilder::new(&mut context.func, fn_builder_ctx);
        let block0 = builder.create_block();

        builder.append_block_params_for_function_params(block0);
        builder.switch_to_block(block0);
        builder.seal_block(block0);

        let (vmctx_ptr_val, caller_vmctx_ptr_val, callee_value, values_vec_ptr_val) = {
            let params = builder.func.dfg.block_params(block0);
            (params[0], params[1], params[2], params[3])
        };

        // Load the argument values out of `values_vec`.
        let mflags = ir::MemFlags::trusted();
        let callee_args = signature
            .params
            .iter()
            .enumerate()
            .map(|(i, r)| {
                match i {
                    0 => vmctx_ptr_val,
                    1 => caller_vmctx_ptr_val,
                    _ =>
                    // i - 2 because vmctx and caller vmctx aren't passed through `values_vec`.
                    {
                        builder.ins().load(
                            r.value_type,
                            mflags,
                            values_vec_ptr_val,
                            ((i - 2) * value_size) as i32,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        let new_sig = builder.import_signature(signature.clone());

        let call = builder
            .ins()
            .call_indirect(new_sig, callee_value, &callee_args);

        let results = builder.func.dfg.inst_results(call).to_vec();

        // Store the return values into `values_vec`.
        let mflags = ir::MemFlags::trusted();
        for (i, r) in results.iter().enumerate() {
            builder
                .ins()
                .store(mflags, *r, values_vec_ptr_val, (i * value_size) as i32);
        }

        builder.ins().return_(&[]);
        builder.finalize()
    }

    let mut code_buf = Vec::new();
    let mut reloc_sink = TrampolineRelocSink {};
    let mut trap_sink = binemit::NullTrapSink {};
    let mut stackmap_sink = binemit::NullStackmapSink {};
    context
        .compile_and_emit(
            isa,
            &mut code_buf,
            &mut reloc_sink,
            &mut trap_sink,
            &mut stackmap_sink,
        )
        .map_err(|error| CompileError::Codegen(pretty_error(&context.func, Some(isa), error)))?;

    let unwind_info = compiled_function_unwind_info(isa, &context);
    // let address_map = get_function_address_map(&context, input, code_buf.len(), isa);
    let address_map = FunctionAddressMap::default(); // get_function_address_map(&context, input, code_buf.len(), isa);

    Ok(CompiledFunction {
        body: code_buf,
        jt_offsets: transform_jump_table(context.func.jt_offsets),
        unwind_info,
        address_map,
        relocations: vec![],
        traps: vec![],
    })
}