use combine::ParseError;
use combine::Parser;
use combine::Stream;
use itertools;

use feature::{BoxedTestCase, TestCase, TestContext};

struct Scenario<TC: TestContext> {
    name: String,
    steps: Vec<Box<Step<TC>>>,
}

/// A specific step which makes up a test context. Users should create there own
/// implementations of this trait, which are returned by their step parsers.
pub trait Step<C: TestContext> {
    fn eval(&self, &mut C) -> bool;
}

impl<C: TestContext> TestCase<C> for Scenario<C> {
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Execute a scenario by creating a new test context, then running each
    /// step in order with mutable access to the context.
    fn eval(&self, mut context: C) -> (bool, C) {
        // let mut ctx = TC::new();
        for s in self.steps.iter() {
            if !s.eval(&mut context) {
                return (false, context);
            }
        }

        (true, context)
    }
}

pub struct BoxedStep<C: TestContext> {
    pub val: Box<Step<C>>,
}

fn scenario_block_parser<I, TC, P>(
    prefix: &'static str,
    inner: P,
) -> impl Parser<Input = I, Output = Vec<BoxedStep<TC>>>
where
    TC: TestContext + 'static,
    P: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    use combine::char::{newline, string};
    use combine::{many, token};

    let first_line = (string(prefix), token(' '), inner.clone(), newline()).map(|t| t.2);
    let and_line = (string("And "), inner, newline()).map(|t| t.1);
    (first_line, many(and_line)).map(|(first, mut ands): (BoxedStep<TC>, Vec<BoxedStep<TC>>)| {
        ands.insert(0, first);
        ands
    })
}

/// A `TestCase` parser for classic Cucumber-style Scenarios; this parser (or a
/// composition thereof) should be passed to feature::parser.
///
/// # Arguments
///
/// * `given`, `when`, `then` : User-defined parsers to parse and produce
/// `Step`s out of the text after `Given`, `When`, and `Then`, respectively.
pub fn parser<I, TC, GP, WP, TP>(
    given: GP,
    when: WP,
    then: TP,
) -> impl Parser<Input = I, Output = BoxedTestCase<TC>>
where
    TC: TestContext + 'static,
    GP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    WP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    TP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    use combine::char::{newline, string};
    use combine::{many, none_of, optional, Parser};

    let scenario_prefix = || string("Scenario: ");

    let non_newline = || none_of("\r\n".chars());

    // Parse until a newline; return everything before the newline character.
    let until_eol = || (many(non_newline()), newline()).map(|(s, _): (String, _)| s);

    // Parse a line with the given prefix
    let prefixed_line = |prefix| (prefix, until_eol()).map(|(_, text): (_, String)| text);

    let givens = scenario_block_parser("Given", given);
    let whens = scenario_block_parser("When", when);
    let thens = scenario_block_parser("Then", then);

    let steps = (
        optional(givens).map(|o| o.unwrap_or(vec![])),
        optional(whens).map(|o| o.unwrap_or(vec![])),
        optional(thens).map(|o| o.unwrap_or(vec![])),
    ).map(|(g, w, t)| itertools::concat(vec![g, w, t]));

    let scenario = (prefixed_line(scenario_prefix()), steps).map(
        |(name, steps): (String, Vec<BoxedStep<TC>>)| Scenario {
            name: name,
            steps: steps.into_iter().map(|s| s.val).collect(),
        },
    );

    scenario.map(|sc| BoxedTestCase { val: Box::new(sc) })
}

#[cfg(test)]
mod tests {
    use super::*;

    use feature;
    use combine::stream::state::State;

    /// The sample test case just records each step as it runs
    struct SampleTestContext {
        executed_steps: Vec<u32>,
    }

    impl TestContext for SampleTestContext {
        fn new() -> SampleTestContext {
            SampleTestContext {
                executed_steps: vec![],
            }
        }
    }

    struct SampleStep {
        num: u32,
    }

    impl Step<SampleTestContext> for SampleStep {
        fn eval(&self, context: &mut SampleTestContext) -> bool {
            context.executed_steps.push(self.num);
            true
        }
    }

    #[test]
    fn scenario() {
        let s = "Feature: my feature\n\
                 \n\
                 Scenario: One\n\
                 Given G1\n\
                 When W1\n\
                 Then T1\n\
                 \n\
                 Scenario: Two\n\
                 Given G2\n\
                 When W2\n\
                 Then T2\n";

        use combine::char::digit;
        use combine::token;

        let num_digit = || digit().map(|c| c.to_digit(10).unwrap());
        let given = struct_parser! { SampleStep { _: token('G'), num: num_digit() } };
        let when = struct_parser! { SampleStep { _: token('W'), num: num_digit() } };
        let then = struct_parser! { SampleStep { _: token('T'), num: num_digit() } };

        let (feat, remaining) = feature::parser(parser(
            given.map(|x| BoxedStep { val: Box::new(x) }),
            when.map(|x| BoxedStep { val: Box::new(x) }),
            then.map(|x| BoxedStep { val: Box::new(x) }),
        )).easy_parse(State::new(s))
            .unwrap();
        println!("End state: {:#?}", remaining);

        assert_eq!(feat.name, "my feature".to_string());
        assert_eq!(feat.comment, "".to_string());
        assert_eq!(feat.test_cases.len(), 2);
        assert_eq!(feat.test_cases[0].name(), "One".clone());
        assert_eq!(feat.test_cases[1].name(), "Two".clone());

        let (pass, ctx) = feat.eval();
        assert!(pass);
        assert_eq!(ctx.executed_steps, vec![1, 1, 1, 2, 2, 2]);
    }
}
