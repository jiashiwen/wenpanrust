pub trait Base {
    fn say(&self) -> String;
}

pub struct AFromBase {
    content: String,
}

impl Base for AFromBase {
    fn say(&self) -> String {
        self.content.clone()
    }
}

pub struct BFromBase {
    text: String,
}

impl Base for BFromBase {
    fn say(&self) -> String {
        self.text.clone()
    }
}

pub struct AddTowBase<'a> {
    a: &'a mut dyn Base,
    b: &'a mut dyn Base,
}

impl<'a> AddTowBase<'a> {
    fn add(self) -> String {
        let result = self.a.say() + &self.b.say();
        result
    }
}

fn main() {
    let mut a = AFromBase {
        content: "baseA".to_string(),
    };

    let mut b = BFromBase {
        text: "baseB".to_string(),
    };

    let addtow = AddTowBase {
        a: &mut a,
        b: &mut b,
    };
    let r = addtow.add();
    println!("{}", r);
}
