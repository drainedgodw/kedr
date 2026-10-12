//! A small garden. A quiet place to grow an idea.

mod garden;
use garden::{Garden, Season};

fn main() {
    let mut garden = Garden::new("Northern lights");
    garden.plant("Cedar", Season::Spring);
    garden.plant("Birch", Season::Autumn);

    for tree in garden.trees() {
        println!("{} grows at its own pace.", tree.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_garden_remembers_its_trees() {
        let mut garden = Garden::new("Home");
        garden.plant("Cedar", Season::Spring);
        assert_eq!(garden.trees().len(), 1);
    }
}
