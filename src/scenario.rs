use combine::ParseError;
use combine::Parser;
use combine::Stream;
use itertools;

use feature::{TestCase, TestContext};

struct Scenario<TC: TestContext> {
    name: String,
    steps: Vec<Box<Step<TC>>>,
}

/// A specific step which makes up a test context. Users should create there own
/// implementations of this trait, which are returned by their step parsers.
trait Step<C: TestContext> {
    fn eval(&self, &mut C) -> bool;
}

impl<C: TestContext> TestCase<C> for Scenario<C> {
    /// Execute a scenario by creating a new test context, then running each
    /// step in order with mutable access to the context.
    fn eval(&self, mut context: C) -> (bool, C) {
        // let mut ctx = TC::new();
        for s in self.steps.iter() {
            if !s.eval(&mut context) {
                return (false, context)
            }
        }

        (true, context)
    }
}

/// A `TestCase` parser for classic Cucumber-style Scenarios; this parser (or a
/// composition thereof) should be passed to feature::parser.
///
/// # Arguments
///
/// * `given`, `when`, `then` : User-defined parsers to parse and produce
/// `Step`s out of the text after `Given`, `When`, and `Then`, respectively.
fn parser<I, TC, SP>(given: SP, when: SP, then: SP) -> impl Parser<Input = I, Output = Scenario<TC>>
where
    TC: TestContext,
    SP: Parser<Input = I, Output = Box<Step<TC>>>,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    use combine::char::{newline, string};
    use combine::{many, many1, none_of, Parser};

    let scenario_prefix = || string("Scenario: ");

    let non_newline = || none_of("\r\n".chars());

    // Parse until a newline; return everything before the newline character.
    let until_eol = || (many(non_newline()), newline()).map(|(s, _): (String, _)| s);

    // Parse a line with the given prefix
    let prefixed_line = |prefix| (prefix, until_eol()).map(|(_, text): (_, String)| text);

    let prefixed = |prefix, p| (string(prefix), p).map(|t| t.1);

    let steps = (
        many1(prefixed("Given ", given)),
        many(prefixed("When ", when)),
        many(prefixed("Then ", then)),
    ).map(|(g, w, t)| itertools::concat(vec![g, w, t]));

    let scenario = (prefixed_line(scenario_prefix()), steps).map(
        |(name, steps): (String, Vec<Box<Step<TC>>>)| Scenario {
            name: name,
            steps: steps,
        },
    );

    scenario
}
