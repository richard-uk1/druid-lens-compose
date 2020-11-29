//! This is scratch space for working out the code we want to generate.
use druid::{
    text::{ArcStr, TextStorage},
    Data, Lens,
};

#[derive(Clone, Data, Lens)]
struct Root {
    name: ArcStr,
}

#[derive(Clone, Data)]
pub struct MyData<Text> {
    text: Text,
}

impl<Text> MyData<Text> {
    pub fn lens_builder<L1>() -> MyDataLens<L1> {
        MyDataLens { text: None }
    }
}

pub struct MyDataLens<L1> {
    text: Option<L1>,
}

const _: () = {
    impl<L1> MyDataLens<L1> {
        pub fn text(mut self, text: L1) -> Self {
            self.text = Some(text);
            self
        }

        pub fn build<T, Text>(self) -> impl Lens<T, MyData<Text>>
        where
            Text: Data + Clone,
            L1: Lens<T, Text>,
        {
            ComposeLens {
                text: self.text.unwrap(),
            }
        }
    }

    struct ComposeLens<L1> {
        text: L1,
    }

    impl<T, Text, L1> Lens<T, MyData<Text>> for ComposeLens<L1>
    where
        Text: Data + Clone,
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
};

fn use_inner<Text>(val: &Text)
where
    Text: TextStorage,
{
    println!("{}", val.as_str());
}

fn main() {
    // Need to annotate MyData will all type parameters: the solver isn't clever enough to do this
    // for us yet.
    let lens = MyData::<ArcStr>::lens_builder().text(Root::name).build();
    let mut root = Root {
        name: ArcStr::from("test"),
    };

    lens.with_mut(&mut root, |v| {
        use_inner(&v.text);
        v.text = "test2".into();
    });
    assert_eq!(&*root.name, "test2");
}
