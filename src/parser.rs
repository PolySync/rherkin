use ast::{Step, TestContext, TestCase, Feature, Scenario};
use itertools;

use combine::ParseError;
use combine::Parser;
use combine::Stream;

use combine::char::{newline, string};
use combine::{many, many1, sep_by, optional, token};
use parse_utils::{line_block, until_eol, eol};

pub struct BoxedStep<C: TestContext> {
    pub val: Box<Step<C>>,
}

fn scenario_block<I, TC, P>(
    prefix: &'static str,
    inner: P,
) -> impl Parser<Input = I, Output = Vec<BoxedStep<TC>>>
where
    TC: TestContext + 'static,
    P: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let first_line = (string(prefix), token(' '), inner.clone(), eol()).map(|t| t.2);
    let and_line = (string("And "), inner, eol()).map(|t| t.1);
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
pub fn scenario<I, TC, GP, WP, TP>(
    given: GP,
    when: WP,
    then: TP,
) -> impl Parser<Input = I, Output = TestCase<TC>>
where
    TC: TestContext + 'static,
    GP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    WP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    TP: Parser<Input = I, Output = BoxedStep<TC>> + Clone,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let givens = scenario_block("Given", given);
    let whens = scenario_block("When", when);
    let thens = scenario_block("Then", then);

    let steps = (
        optional(givens).map(|o| o.unwrap_or(vec![])),
        optional(whens).map(|o| o.unwrap_or(vec![])),
        optional(thens).map(|o| o.unwrap_or(vec![])),
    ).map(|(g, w, t)| itertools::concat(vec![g, w, t]));

    let scenario = struct_parser! {
        Scenario {
            _: string("Scenario: "),
            name: until_eol(),
            steps: steps.map(|x| x.into_iter().map(|s| s.val).collect()),
        }
    };

    scenario.map(|s| TestCase::Scenario(s))
}


/// Construct a feature file parser, built around a test-case parser.
pub fn feature<I, C, TP>(test_case_parser: TP) -> impl Parser<Input = I, Output = Feature<C>>
where
    C: TestContext,
    TP: Parser<Input = I, Output = TestCase<C>>,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let blank_lines = || many1::<Vec<_>, _>(newline());

    let test_cases = sep_by(test_case_parser, blank_lines());

    struct_parser! {
        Feature {
            _: optional(blank_lines()),
            _: string("Feature: "),
            name: until_eol(),
            comment: line_block(),
            _: blank_lines(),
            test_cases: test_cases
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
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

    fn do_parse(s: &str) -> Feature<SampleTestContext> {
        use combine::char::digit;
        use combine::token;

        let num_digit = || digit().map(|c| c.to_digit(10).unwrap());
        let given = struct_parser! { SampleStep { _: token('G'), num: num_digit() } };
        let when = struct_parser! { SampleStep { _: token('W'), num: num_digit() } };
        let then = struct_parser! { SampleStep { _: token('T'), num: num_digit() } };

        let (feat, remaining) = feature(scenario(
            given.map(|x| BoxedStep { val: Box::new(x) }),
            when.map(|x| BoxedStep { val: Box::new(x) }),
            then.map(|x| BoxedStep { val: Box::new(x) }),
        )).easy_parse(State::new(s))
            .unwrap();

        println!("End state: {:#?}", remaining);

        feat
    }

    #[test]
    fn test_parse() {
        let feat = do_parse(r"
Feature: my feature
one
two

Scenario: One
Given G1
When W1
Then T1

Scenario: Two
Given G2
When W2
Then T2");

        assert_eq!(feat.name, "my feature".to_string());
        assert_eq!(feat.comment, "one\ntwo".to_string());
        assert_eq!(feat.test_cases.len(), 2);
        assert_eq!(feat.test_cases[0].name(), "One".clone());
        assert_eq!(feat.test_cases[1].name(), "Two".clone());

        let (pass, ctx) = feat.eval();
        assert!(pass);
        assert_eq!(ctx.executed_steps, vec![1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn test_parse_extra_whitespace() {
        let feat = do_parse(r"
Feature: my feature

Scenario: One
Given G1
When W1
Then T1

Scenario: Two
Given G2
When W2
Then T2
");

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
