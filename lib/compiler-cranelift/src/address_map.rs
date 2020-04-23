use cranelift_codegen::{isa, Context};
use wasmer_compiler::{FunctionAddressMap, FunctionBodyData, InstructionAddressMap, SourceLoc};

pub fn get_function_address_map<'data>(
    context: &Context,
    data: &FunctionBodyData<'data>,
    body_len: usize,
    isa: &dyn isa::TargetIsa,
) -> FunctionAddressMap {
    let mut instructions = Vec::new();

    let func = &context.func;
    let mut blocks = func.layout.blocks().collect::<Vec<_>>();
    blocks.sort_by_key(|block| func.offsets[*block]); // Ensure inst offsets always increase

    let encinfo = isa.encoding_info();
    for block in blocks {
        for (offset, inst, size) in func.inst_offsets(block, &encinfo) {
            let srcloc = func.srclocs[inst];
            instructions.push(InstructionAddressMap {
                srcloc: SourceLoc::new(srcloc.bits()),
                code_offset: offset as usize,
                code_len: size as usize,
            });
        }
    }

    // Generate artificial srcloc for function start/end to identify boundary
    // within module. Similar to FuncTranslator::cur_srcloc(): it will wrap around
    // if byte code is larger than 4 GB.
    let start_srcloc = SourceLoc::new(data.module_offset as u32);
    let end_srcloc = SourceLoc::new((data.module_offset + data.data.len()) as u32);

    FunctionAddressMap {
        instructions,
        start_srcloc,
        end_srcloc,
        body_offset: 0,
        body_len,
    }
}