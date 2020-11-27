//! This is scratch space for workout out the code we want to generate.
use druid::{
    text::{ArcStr, TextStorage},
    Data, Lens,
};

#[derive(Clone, Data, Lens)]
struct Root {
    name: ArcStr,
}

#[derive(Clone, Data)]
struct MyData<Text> {
    text: Text,
}

impl<Text> MyData<Text>
where
    Text: Data + Clone,
{
    fn compose_lens<T>(text: impl Lens<T, Text>) -> impl Lens<T, MyData<Text>> {
        struct ComposeLens<L1> {
            text: L1,
        }

        impl<T, Text, L1> Lens<T, MyData<Text>> for ComposeLens<L1>
        where
            Text: Clone + Data,
            L1: Lens<T, Text>,
        {
            fn with<V, F: FnOnce(&MyData<Text>) -> V>(&self, data: &T, f: F) -> V {
                let text = self.text.with(data, |v| v.clone());
                let _widget_data = MyData { text };
                f(&_widget_data)
            }
            fn with_mut<V, F: FnOnce(&mut MyData<Text>) -> V>(&self, data: &mut T, f: F) -> V {
                let text = self.text.with(data, |v| v.clone());
                let mut _widget_data = MyData { text };
                let output = f(&mut _widget_data);
                let MyData { text } = _widget_data;
                self.text.with_mut(data, |v| {
                    if !Data::same(v, &text) {
                        *v = text;
                    }
                });
                output
            }
        }
        ComposeLens { text }
    }
}

fn use_inner<Text>(val: &Text)
where
    Text: TextStorage,
{
    println!("{}", val.as_str());
}

fn main() {
    let lens = MyData::compose_lens(Root::name);
    let mut root = Root {
        name: "test".into(),
    };

    lens.with_mut(&mut root, |v| {
        use_inner(&v.text);
        v.text = "test2".into();
    });
    assert_eq!(&*root.name, "test2");
}
