use vss;

#[test]
fn test_string_loading() {
    let s = vss::load_as_string("ed_config.json");
    assert!(s.len() > 0);
    assert_eq!(s.as_str()[0..1], *"{");
    assert_eq!(s.as_str()[s.len() - 2..s.len() - 1], *"}");
}

#[test]
fn test_load_texture_from_bytes() {
    let width = 2 as u16;
    let height = 2 as u16;

    let bytes = vec![0; (width * height * 4) as usize];

    let mut factory = initialise_opengl(width as u32, height as u32).factory;

    let res = vss::load_texture_from_bytes(
        &mut factory,
        bytes.into_boxed_slice(),
        width as u32,
        height as u32,
    );

    assert!(res.is_ok());

    let (tex, _) = res.unwrap();

    assert_eq!(tex.get_info().to_image_info(0).width, width);
    assert_eq!(tex.get_info().to_image_info(0).height, height);
    assert_eq!(tex.get_info().to_image_info(0).xoffset, 0);
}

#[test]
fn test_update_texture() {
    let mut width = 2 as u16;
    let mut height = 2 as u16;

    let bytes = vec![0; (width * height * 4) as usize];

    let mut bundle = initialise_opengl(width as u32, height as u32);
    let mut factory = bundle.factory.clone();
    let encoder = &mut bundle.encoder;
    let res = vss::load_texture_from_bytes(
        &mut factory,
        bytes.into_boxed_slice(),
        width as u32,
        height as u32,
    );

    assert!(res.is_ok());

    let (tex, _) = res.unwrap();

    assert_eq!(tex.get_info().to_image_info(0).width, width);

    width = 8; //change with no new value
    height = 8; //change with no new value

    let size = [width as u16, height as u16];
    let offset = [0, 0];

    let bytes = vec![0; (width * height * 4) as usize];

    vss::update_texture(encoder, &tex, size, offset, &*bytes);

    assert_eq!(tex.get_info().to_image_info(0).width, 2); //width should have changed
}

#[test]
fn test_load_single_channel_texture_from_bytes() {
    let width = 2 as u16;
    let height = 2 as u16;

    let bytes = vec![0; (width * height) as usize]; // notice not times 4

    let mut factory = initialise_opengl(width as u32, height as u32).factory;

    let res = vss::load_single_channel_texture_from_bytes(
        &mut factory,
        bytes.into_boxed_slice(),
        width as u32,
        height as u32,
    );

    assert!(res.is_ok());

    let (tex, _) = res.unwrap();

    assert_eq!(tex.get_info().to_image_info(0).width, width);
    assert_eq!(tex.get_info().to_image_info(0).height, height);
    assert_eq!(tex.get_info().to_image_info(0).xoffset, 0);
}
