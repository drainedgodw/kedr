#[derive(Debug, Clone, Copy)]
pub enum Season { Spring, Autumn }

pub struct Tree { pub name: String, pub season: Season }
pub struct Garden { pub name: String, trees: Vec<Tree> }

impl Garden {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), trees: Vec::new() }
    }
    pub fn plant(&mut self, name: &str, season: Season) {
        self.trees.push(Tree { name: name.into(), season });
    }
    pub fn trees(&self) -> &[Tree] { &self.trees }
}
