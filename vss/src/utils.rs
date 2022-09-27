use std::io::Cursor;
use std::path::Path;

// from https://stackoverflow.com/a/42186553
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::std::mem::size_of::<T>(),
    )
}

#[cfg(not(target_os = "android"))]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::File;
    use std::io::Read;

    let mut buf = Vec::new();
    let full_path = &Path::new("").join(&path);
    let mut file = match File::open(&full_path) {
        Ok(file) => file,
        Err(err) => {
            panic!("Can`t open file '{}' ({})", full_path.display(), err);
        }
    };
    match file.read_to_end(&mut buf) {
        Ok(_) => Cursor::new(buf),
        Err(err) => {
            panic!("Can`t read file '{}' ({})", full_path.display(), err);
        }
    }
}

#[cfg(target_os = "android")]
pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use android_glue;

    let filename = path.as_ref().to_str().expect("Can`t convert Path to &str");
    match android_glue::load_asset(filename) {
        Ok(buf) => Cursor::new(buf),
        Err(_) => panic!("Can`t load asset '{}'", filename),
    }
}
