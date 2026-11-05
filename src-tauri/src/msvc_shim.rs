//! ONNX Runtime's prebuilt Windows libraries are compiled against a recent MSVC STL, which references
//! vectorised helpers that older Build Tools (< 14.44) don't ship. Providing the symbols it needs
//! lets fuzz link with any supported Visual Studio 2022 toolchain.

use std::ffi::c_void;

/// `std::find_first_of` for trivially comparable elements of type `T`:
/// first element of [first1,last1) equal to any element of [first2,last2), else `last1`.
unsafe fn find_first_of<T: PartialEq + Copy>(first1: *const c_void, last1: *const c_void, first2: *const c_void, last2: *const c_void) -> *const c_void {
    let sz = std::mem::size_of::<T>();
    let n1 = (last1 as usize).saturating_sub(first1 as usize) / sz;
    let n2 = (last2 as usize).saturating_sub(first2 as usize) / sz;
    let hay = std::slice::from_raw_parts(first1 as *const T, n1);
    let needles = std::slice::from_raw_parts(first2 as *const T, n2);
    match hay.iter().position(|c| needles.contains(c)) {
        Some(i) => (first1 as *const T).add(i) as *const c_void,
        None => last1,
    }
}

macro_rules! shim {
    ($name:ident, $t:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(a: *const c_void, b: *const c_void, c: *const c_void, d: *const c_void) -> *const c_void {
            find_first_of::<$t>(a, b, c, d)
        }
    };
}
shim!(__std_find_first_of_trivial_pos_1, u8);
shim!(__std_find_first_of_trivial_pos_2, u16);
shim!(__std_find_first_of_trivial_pos_4, u32);
shim!(__std_find_first_of_trivial_pos_8, u64);
