use crate::protocol::xproto::{ModMask, SendEventDest, VisualClass};

#[test]
fn test_enum_debug() {
    assert_eq!("TRUE_COLOR", format!("{:?}", VisualClass::TRUE_COLOR));
    assert_eq!("TrueColor", format!("{:#?}", VisualClass::TRUE_COLOR));
    assert_eq!(
        "POINTER_WINDOW",
        format!("{:?}", SendEventDest::POINTER_WINDOW)
    );
    assert_eq!(
        "PointerWindow",
        format!("{:#?}", SendEventDest::POINTER_WINDOW)
    );
    assert_eq!("ITEM_FOCUS", format!("{:?}", SendEventDest::ITEM_FOCUS));
    assert_eq!("ItemFocus", format!("{:#?}", SendEventDest::ITEM_FOCUS));
}

#[test]
fn test_bitmask_debug() {
    assert_eq!("SHIFT", format!("{:?}", ModMask::SHIFT));
    assert_eq!("Shift", format!("{:#?}", ModMask::SHIFT));
    assert_eq!(
        "SHIFT | LOCK",
        format!("{:?}", ModMask::SHIFT | ModMask::LOCK)
    );
    assert_eq!(
        "Shift | Lock",
        format!("{:#?}", ModMask::SHIFT | ModMask::LOCK)
    );
    assert_eq!("0", format!("{:?}", ModMask::from(0u8)));
}
