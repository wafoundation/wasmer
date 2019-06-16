//! Create, read, write, grow, destroy memory of an instance.

use crate::{
    //error::Error, error::RuntimeError,
    error::update_last_error,
    wasmer_limits_t,
    wasmer_result_t,
};
use std::cell::Cell;
use wasmer_runtime::Memory;
use wasmer_runtime_core::{
    types::MemoryDescriptor,
    units::{Bytes, Pages},
};

#[repr(C)]
#[derive(Clone)]
pub struct wasmer_memory_t;

/// Creates a new Memory for the given descriptor and initializes the given
/// pointer to pointer to a pointer to the new memory.
///
/// The caller owns the object and should call `wasmer_memory_destroy` to free it.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[no_mangle]
pub unsafe extern "C" fn wasmer_memory_new(
    memory: *mut *mut wasmer_memory_t,
    limits: wasmer_limits_t,
) -> wasmer_result_t {
    let max = if limits.max.has_some {
        Some(Pages(limits.max.some))
    } else {
        None
    };
    let desc = MemoryDescriptor {
        minimum: Pages(limits.min),
        maximum: max,
        shared: false,
    };
    let result = Memory::new(desc);
    let new_memory = match result {
        Ok(memory) => memory,
        Err(error) => {
            update_last_error(error);
            return wasmer_result_t::WASMER_ERROR;
        }
    };
    *memory = Box::into_raw(Box::new(new_memory)) as *mut wasmer_memory_t;
    wasmer_result_t::WASMER_OK
}

/// Grows a Memory by the given number of pages.
///
/// Returns `wasmer_result_t::WASMER_OK` upon success.
///
/// Returns `wasmer_result_t::WASMER_ERROR` upon failure. Use `wasmer_last_error_length`
/// and `wasmer_last_error_message` to get an error message.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_grow(memory: *mut wasmer_memory_t, delta: u32) -> wasmer_result_t {
    let memory = unsafe { &*(memory as *mut Memory) };
    let delta_result = memory.grow(Pages(delta));
    match delta_result {
        Ok(_) => wasmer_result_t::WASMER_OK,
        Err(grow_error) => {
            update_last_error(grow_error);
            wasmer_result_t::WASMER_ERROR
        }
    }
}

/// Returns the current length in pages of the given memory
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_length(memory: *const wasmer_memory_t) -> u32 {
    let memory = unsafe { &*(memory as *const Memory) };
    let Pages(len) = memory.size();
    len
}

/// Gets the start pointer to the bytes within a Memory
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_data(mem: *const wasmer_memory_t) -> *mut u8 {
    let memory = unsafe { &*(mem as *const Memory) };
    memory.view::<u8>()[..].as_ptr() as *mut Cell<u8> as *mut u8
}

/// Gets the size in bytes of a Memory
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_data_length(mem: *mut wasmer_memory_t) -> u32 {
    let memory = mem as *mut Memory;
    let Bytes(len) = unsafe { (*memory).size().bytes() };
    len as u32
}

/// Copies an input buffer into Memory.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_data_copy(
    mem: *mut wasmer_memory_t,
    mem_offset: u32,
    buffer: *const u8,
    buffer_len: u32,
) -> wasmer_result_t {
    let last_mem_offset = wasmer_memory_data_length(mem) - 1;
    let last_mem_copy_offset = mem_offset + (buffer_len - 1);

    if last_mem_copy_offset > last_mem_offset {
        // let err = RuntimeError::from("Memory illegal accesss: out of bounds");
        // update_last_error(Error::(RuntimeError(err));

        return wasmer_result_t::WASMER_ERROR;
    }

    let mem_start_ptr: *mut u8 = wasmer_memory_data(mem);

    unsafe {
        let mem_ptr: *mut u8 = mem_start_ptr.offset(mem_offset as isize);

        std::ptr::copy(buffer, mem_ptr, buffer_len as usize);
    }

    wasmer_result_t::WASMER_OK
}

/// Frees memory for the given Memory
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn wasmer_memory_destroy(memory: *mut wasmer_memory_t) {
    if !memory.is_null() {
        unsafe { Box::from_raw(memory as *mut Memory) };
    }
}
