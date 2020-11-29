use druid::{ArcStr, Data, Lens};
use druid_lens_compose::ComposeLens;

#[derive(Clone, Data, ComposeLens)]
pub struct MyData<T2> {
    field1: T2,
    field2: u64,
}

#[derive(Clone, Data, Lens)]
pub struct Root<T> {
    f1: T,
    f2: u64,
}

#[test]
fn pass() {
    let lens = MyData::<ArcStr>::lens_builder()
        .field1(Root::<ArcStr>::f1)
        .field2(Root::<ArcStr>::f2)
        .build::<_, ArcStr>();

    let mut root = Root {
        f1: ArcStr::from("test"),
        f2: 0,
    };

    lens.with_mut(&mut root, |v| v.field1 = ArcStr::from("test2"));
    assert_eq!(&*root.f1, "test2");
}
