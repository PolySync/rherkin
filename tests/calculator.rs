#[macro_use]
extern crate combine;
use combine::{Parser, many1};
use combine::easy::Error;
use combine::stream::state::State;
use combine::char::{string, digit};

extern crate rherkin;
use rherkin::feature::{self, TestContext};
use rherkin::scenario::{self, Step, BoxedStep};

// An rpn calculator, something we can write tests for.
#[derive(Debug)]
pub struct Calculator {
    /// The digits that are currently being entered
    pub current: Vec<u32>,

    /// The data stack
    pub stack: Vec<u32>
}

#[derive(Clone, Debug)]
pub enum Button {
    Number(u32),
    Enter,
    Plus,
    Minus,
    Times,
    Divide
}

impl Calculator {
    fn press(&mut self, button: &Button) -> bool {
        println!("State: {:?}", self);

        use Button::*;

        match button {
            Number(ref n) => {
                if *n <= 9 {
                    self.current.push(*n);
                    return true
                } else {
                    return false
                }
            },

            Enter => {
                let mut n: u32 = 0;
                let mut factor = 1;
                for digit in self.current.iter().rev() {
                    n += digit * factor;
                    factor *= 10;
                }

                self.stack.push(n);
                self.current.clear();
                return true;
            },

            Plus => {
                self.press(&Button::Enter);

                let a = match self.stack.pop() {
                    Some(x) => x,
                    None => return false,
                };

                let b = match self.stack.pop() {
                    Some(x) => x,
                    None => return false,
                };

                self.stack.push(a + b);
                return true;
            },

            _ => false
        }
    }
}

impl TestContext for Calculator {
    fn new() -> Calculator {
        Calculator {
            current: vec!(),
            stack: vec!()
        }
    }
}

mod steps {
    use super::*;

    pub struct Clear { }
    impl Step<Calculator> for Clear {
        fn eval(&self, calc: &mut Calculator) -> bool {
            println!("Clear");
            calc.current = vec!();
            calc.stack = vec!();
            true
        }
    }

    pub struct Press { pub button: Button }
    impl Step<Calculator> for Press {
        fn eval(&self, calc: &mut Calculator) -> bool {
            println!("Press {:?}", self.button);
            calc.press(&self.button)
        }
    }

    pub struct CheckDisplay { pub expected: String }
    impl Step<Calculator> for CheckDisplay {
        fn eval(&self, calc: &mut Calculator) -> bool {
            let actual = calc.stack.last();
            println!("Check display: expected {:?}, actual {:#?}", self.expected, actual);
            match actual {
                Some(n) => format!("{}", n) == self.expected,
                None => false
            }
        }
    }

}


#[test]
fn scenarios() {
    let spec = r#"
Feature: RPN Calculator Arithmetic
The calculator supports basic addition, subtraction, multiplication, and
division operations.

Scenario: basic addition
Given a fresh calculator
When I press 1
And I press enter
And I press 1
And I press plus
Then the display should read 2"#;

    use steps::*;

    let clear = struct_parser! {
        Clear {
            _: string("a fresh calculator")
        }
    };

    let press = struct_parser! {
        Press {
            _: string("I press "),
            button: choice! {
                string("enter").map(|_| Button::Enter),
                string("plus").map(|_| Button::Plus),
                string("minus").map(|_| Button::Minus),
                string("times").map(|_| Button::Times),
                string("divide").map(|_| Button::Divide),
                digit().and_then(|ch| match ch.to_digit(10) {
                    Some(n) => Ok(Button::Number(n)),
                    None => Err(Error::Unexpected(ch.into()))
                })
            }
        }
    };

    let check_display = struct_parser! {
        CheckDisplay {
            _: string("the display should read "),
            expected: many1(digit())
        }
    };

    let given = choice! { clear };
    let when = choice! { press };
    let then = choice! { check_display };

    let mut p =
        feature::parser(
            scenario::parser(
                given.map(|x| BoxedStep { val: Box::new(x) }),
                when.map(|x| BoxedStep { val: Box::new(x) }),
                then.map(|x| BoxedStep { val: Box::new(x) })));

    let (f, _) = p.easy_parse(State::new(spec)).unwrap();

    let (success, _) = f.eval();
    assert!(success);
}

