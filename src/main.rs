
use std::vec;

struct Tree {
    color: u8,
    children: Vec<Tree>,
}

struct Set {
    num_colors: u8,
    trees: Vec<Tree>,
}

struct QueueEntry<'a> {
    tree: &'a mut Tree,
    remaining: Vec<u8>,
}

impl Set {
    fn new(num_colors: u8) -> Set {
        let mut output = Set {
            num_colors: num_colors,
            trees: Vec::new(),
        };

        let mut queue = Vec::<QueueEntry>::new();
        for i in 0..num_colors {
            output.trees.push(Tree{
                color: i,
                children: Vec::new(),
            });
        }

        let all_colors: Vec<u8> = (0..num_colors).collect();
        for tree in output.trees.iter_mut() {
            let mut remaining_colors = all_colors.clone();
            remaining_colors.swap_remove(tree.color as usize);

            queue.push(QueueEntry{
                tree: &mut *tree,
                remaining: remaining_colors,
            });
        }

        while !queue.is_empty() {
            let top = queue.pop().unwrap();
            
            for i in top.remaining.iter() {
                top.tree.children.push(Tree{
                    color: *i,
                    children: Vec::new(),
                });
            }

            for tree in top.tree.children.iter_mut() {
                let mut new_remaining = top.remaining.clone();
                new_remaining.swap_remove(
                    new_remaining.iter().position(|&x| tree.color == x).unwrap()
                );

                queue.push(QueueEntry{
                    tree: &mut *tree,
                    remaining: new_remaining,
                });
            }
        }

        return output;
    }
}

struct PartialCombination<'a> {
    tree: &'a Tree,
    partial: Vec<u8>,
}

fn main() {
    
    let initial_set = Set::new(6);

    let mut combinations = Vec::<Vec<u8>>::new();
    let mut queue = Vec::<PartialCombination>::new();

    for tree in initial_set.trees.iter() {
        queue.push(PartialCombination{
            tree: & *tree,
            partial: Vec::new(),
        });
    }

    while !queue.is_empty() {
        let top = queue.pop().unwrap();

        let mut partial = top.partial;
        partial.push(top.tree.color);

        if top.tree.children.is_empty() {
            combinations.push(partial);
        } else {
            for child in top.tree.children.iter() {
                queue.push(PartialCombination{
                    tree: & *child,
                    partial: partial.clone(),
                });
            }
        }
    }

    for combination in combinations.iter() {
        println!("{:?}", *combination);
    }

    println!("\nTotal combinations: {}", combinations.len())
}
