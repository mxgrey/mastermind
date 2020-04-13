
use std::vec;
use std::cmp;
use std::collections::HashSet;
use std::iter::Iterator;
use std::fmt;
use std::sync::{Arc, Mutex, atomic};
use rand::Rng;
use rayon::prelude::*;
use termion::{color, color::Color};

#[derive(Clone)]
struct Set {
    combinations: Vec<Vec<u8>>,
}

struct Subset<'a> {
    original: &'a Set,
    members: HashSet<u32>,
}

trait FilterSet {
    fn apply<'a, I>(
        &self,
        full_set: &Vec<Vec<u8>>,
        index_iter: I)
        -> HashSet<u32>
    where
        I: Iterator<Item = &'a u32>;
}

impl Set {
    fn new(num_colors: u8, num_spaces: u8) -> Set {

        let mut output = Set {
            combinations: Vec::new(),
        };
        
        let mut combination = Vec::<u8>::new();
        for _ in 0..num_spaces {
            combination.push(0);
        }
        output.combinations.push(combination.clone());

        let mut index : usize = 0;
        while (index as u8) < num_spaces {
            let c = &mut combination[index];
            *c += 1;
            if num_colors <= *c {
                *c = 0;
                index += 1;
                continue;
            } else {
                index = 0;
            }

            output.combinations.push(combination.clone());
        }

        return output;
    }

    fn as_subset(&self) -> Subset {
        return Subset {
            original: &self,
            members: (0..self.combinations.len() as u32).collect(),
        };
    }
}

impl<'a> Subset<'a> {
    fn make_subset(&self, filter: &impl FilterSet) -> Subset<'a>
    {
        return Subset {
            original: self.original,
            members: filter.apply(&self.original.combinations, self.members.iter()),
        };
    }
}

struct Representation {
    fg_color: Box<dyn Color>,
    bg_color: Box<dyn Color>,
    text: String,
}

impl fmt::Display for Representation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Err(fmt::Error) = self.fg_color.write_fg(f) {
            return Err(fmt::Error);
        }

        if let Err(fmt::Error) = self.bg_color.write_bg(f) {
            return Err(fmt::Error);
        }

        if let Err(fmt::Error) = f.write_str(&self.text) {
            return Err(fmt::Error);
        }

        let reset = color::Reset{};

        if let Err(fmt::Error) = reset.write_fg(f) {
            return Err(fmt::Error);
        }

        if let Err(fmt::Error) = reset.write_bg(f) {
            return Err(fmt::Error);
        }
        
        return Ok(());
    }
}

struct Style {
    reps: Vec<Representation>,
}

struct Combo<'a, 'b> {
    style: &'a Style,
    combination: &'b Vec<u8>,
}

impl<'a, 'b> Combo<'a, 'b> {
    fn new<'c, 'd>(style: &'c Style, combination: &'d Vec<u8>) -> Combo<'c, 'd> {
        return Combo{
            style: style,
            combination: combination,
        };
    }
}

impl<'a, 'b> fmt::Display for Combo<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in self.combination.iter() {
            if let Err(fmt::Error) = write!(f, " {} ", self.style.reps[*i as usize]) {
                return Err(fmt::Error);
            }
        }

        return Ok(());
    }
}

#[derive(PartialEq, Eq)]
struct Score {
    white: u8,
    black: u8,
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "[white: {} | black: {}]", self.white, self.black);
    }
}

impl Score {
    fn compute(guess: &Vec<u8>, answer: &Vec<u8>) -> Score {
        let mut score = Score{
            white: 0,
            black: 0,
        };

        assert!(guess.len() == answer.len());

        let mut highest_color = 0;
        for i in 0..guess.len() {
            if guess[i] == answer[i] {
                score.black += 1;
            }

            if answer[i] > highest_color {
                highest_color = answer[i];
            }

            if guess[i] > highest_color {
                highest_color = guess[i];
            }
        }

        let mut answer_color_count = Vec::<u8>::new();
        answer_color_count.resize_with((highest_color+1) as usize, || 0);

        let mut guess_color_count = Vec::<u8>::new();
        guess_color_count.resize_with((highest_color+1) as usize, || 0);

        for i in 0..guess.len() {
            guess_color_count[guess[i] as usize] += 1;
            answer_color_count[answer[i] as usize] += 1;
        }

        for i in 0..guess_color_count.len() {
            score.white += cmp::min(guess_color_count[i], answer_color_count[i]);
        }

        score.white -= score.black;

        return score;
    }
}

struct ScoreFilter<'a> {
    score: &'a Score,
    guess: &'a Vec<u8>,
}

impl FilterSet for ScoreFilter<'_> {
    fn apply<'a, I>(
        &self, 
        full_set: &Vec<Vec<u8>>, 
        index_iter: I) -> HashSet<u32>
    where
        I: Iterator<Item = &'a u32>
    {
        let mut output = HashSet::<u32>::new();
        for index in index_iter {
            if Score::compute(self.guess, &full_set[*index as usize]) == *self.score {
                output.insert(*index);
            }
        }

        return output;
    }
}

struct BestChoices {
    candidates: Vec<u32>,
    fewest_eliminations: u32,
}

impl BestChoices {
    fn evaluate(&mut self, candidate: u32, candidate_fewest_eliminations: u32) {
        if self.fewest_eliminations < candidate_fewest_eliminations {
            self.fewest_eliminations = candidate_fewest_eliminations;
            self.candidates.clear();
            self.candidates.push(candidate);
        } else if self.fewest_eliminations == candidate_fewest_eliminations {
            self.candidates.push(candidate);
        }
    }
}

fn decide(remaining_subset: &Subset, style: &Style) -> u32 {

    let all_combinations = &remaining_subset.original.combinations;

    let best_choices_arc = Arc::new(
        Mutex::new(
            BestChoices{
                candidates: Vec::<u32>::new(),
                fewest_eliminations: 0,
            }
        )
    );

    let total_candidates = all_combinations.len();
    let finished_candidates = atomic::AtomicU32::new(0);

    all_combinations.par_iter().enumerate().for_each(|(candidate_index, candidate)| {
        // println!("Testing candidate {}", candidate_index);
        let local_best_choices_arc = Arc::clone(&best_choices_arc);
        let best_fewest_so_far = local_best_choices_arc.lock().unwrap().fewest_eliminations;

        let mut fewest_eliminations = u32::max_value();
        for possible_answer_index in remaining_subset.members.iter() {
            let possible_answer = &all_combinations[*possible_answer_index as usize];
            let mut eliminations: u32 = 0;
            for remaining_index in remaining_subset.members.iter() {
                let remaining = &all_combinations[*remaining_index as usize];
                if Score::compute(candidate, possible_answer) != Score::compute(candidate, remaining) {
                    eliminations += 1;
                }
            }

            if eliminations < fewest_eliminations {
                fewest_eliminations = eliminations;
            }

            if fewest_eliminations < best_fewest_so_far {
                break;
            }
        }

        if best_fewest_so_far <= fewest_eliminations {
            local_best_choices_arc.lock().unwrap().evaluate(
                candidate_index as u32, 
                fewest_eliminations
            );
        }

        let local_finished_candidates = finished_candidates.fetch_add(1u32, atomic::Ordering::Relaxed) + 1u32;
        println!("Progress: {:.2}%", (local_finished_candidates as f64)/(total_candidates as f64)*100f64);
    });

    let best_choices = best_choices_arc.lock().unwrap();
    println!("Best candidates: {:?} | fewest possible eliminations: {}", 
        best_choices.candidates, best_choices.fewest_eliminations);

    for candidate in best_choices.candidates.iter() {
        // println!("{:?}", all_combinations[*candidate as usize]);
        println!(" {}\n", Combo{style: style, combination: &all_combinations[*candidate as usize]});
    }

    for candidate in best_choices.candidates.iter() {
        if remaining_subset.members.contains(candidate) {
            return *candidate;
        }
    }

    return *best_choices.candidates.last().unwrap();
}

fn main() {

    let num_colors = 6;

    let initial_set = Set::new(num_colors, 4);
    let mut remaining_set = initial_set.as_subset();
    let mut rng = rand::thread_rng();

    let style = Style{
        reps: vec![
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::LightRed{}),
                text: String::from("R"),
            },
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::Green{}),
                text: String::from("G"),
            },
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::LightYellow{}),
                text: String::from("Y"),
            },
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::LightBlue{}),
                text: String::from("B"),
            },
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::LightMagenta{}),
                text: String::from("P"),
            },
            Representation{
                fg_color: Box::new(color::Black{}),
                bg_color: Box::new(color::LightCyan{}),
                text: String::from("C"),
            },
        ],
    };

    let answer = vec![
        rng.gen_range(0, num_colors),
        rng.gen_range(0, num_colors),
        rng.gen_range(0, num_colors),
        rng.gen_range(0, num_colors)
    ];

    let initial_guess = vec![0, 0, 1, 1];
    let initial_score = Score::compute(&initial_guess, &answer);
    remaining_set = remaining_set.make_subset(&ScoreFilter{score: &initial_score, guess: &initial_guess});

    for i in 0..8 {
        let guess_index = decide(&remaining_set, &style);
        println!("Guess index: {}", guess_index);
        let choice = &initial_set.combinations[guess_index as usize];

        println!("Guessing: {}", Combo::new(&style, choice));
        
        let score = Score::compute(&choice, &answer);
        println!("Score: {}", score);

        remaining_set = remaining_set.make_subset(&ScoreFilter{score: &score, guess: choice});

        if *choice == answer {
            println!("Solved with {} guesses!", i+1);
            break;
        }
        else {
            println!("Number in remaining set: {}", remaining_set.members.len());
        }
    }

    if remaining_set.members.len() > 1 {
        println!("Remaining combinations:");
        for m in remaining_set.members.iter() {
            println!("{}", Combo::new(&style, &initial_set.combinations[*m as usize]));
        }
        println!("\nNumber of remaining combinations: {}", remaining_set.members.len());
    }

    println!("Correct answer: {}", Combo::new(&style, &answer));
}
