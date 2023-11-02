#![cfg(loom)]

#[test]
fn smoke() {
    loom::model(|| {
        let (p, u) = parking::pair();

        loom::thread::spawn(move || {
            p.park();
        });

        u.unpark();
    });
}
