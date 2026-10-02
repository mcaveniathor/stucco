//! The WebAssembly interface: JavaScript writes a UTF-8 query into memory
//! from `alloc`, calls `generate`, reads the JSON result, and frees both
//! buffers with `dealloc`.

/// Allocates `len` bytes for the caller to fill.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees a buffer from `alloc` or `generate`.
///
/// # Safety
///
/// `ptr` and `len` must come from one `alloc(len)` call or one `generate`
/// result, and be freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller passes a pointer and capacity this module allocated.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
}

/// Generates the playground JSON for the query in `query_ptr..+query_len`,
/// naming the scoped CSS block after the scope in `scope_ptr..+scope_len`.
/// Returns the result's pointer in the high 32 bits and its length in the
/// low 32 bits; free it with `dealloc`.
///
/// # Safety
///
/// Both ranges must be initialised memory from `alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn generate(
    query_ptr: *const u8,
    query_len: usize,
    scope_ptr: *const u8,
    scope_len: usize,
) -> u64 {
    // SAFETY: the caller wrote these ranges after allocating them with `alloc`.
    let (query, scope) = unsafe {
        (
            std::slice::from_raw_parts(query_ptr, query_len),
            std::slice::from_raw_parts(scope_ptr, scope_len),
        )
    };
    let json = crate::generate_json(
        std::str::from_utf8(query).unwrap_or(""),
        std::str::from_utf8(scope).unwrap_or(""),
    );
    let mut bytes = json.into_bytes().into_boxed_slice();
    let (ptr, len) = (bytes.as_mut_ptr(), bytes.len());
    std::mem::forget(bytes);
    ((ptr as u64) << 32) | len as u64
}
