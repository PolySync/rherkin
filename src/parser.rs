use ast::{Feature, Scenario, Step, TestCase, TestContext};
use itertools;

use combine::ParseError;
use combine::Parser;
use combine::Stream;

use combine::char::{newline, string};
use combine::{many, many1, optional, sep_by, token};
use parse_utils::{eol, line_block, until_eol};

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

fn scenario<I, C, GP, WP, TP>(
    prefix: &'static str,
    given: GP,
    when: WP,
    then: TP,
) -> impl Parser<Input = I, Output = Scenario<C>>
where
    C: TestContext + 'static,
    GP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
    WP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
    TP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
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

    struct_parser! {
        Scenario {
            _: string(prefix),
            _: string(":"),
            name: choice!(
                until_eol().map(|s| Some(s.trim().to_string())),
                newline().map(|_| None)
            ),
            steps: steps.map(|x| x.into_iter().map(|s| s.val).collect()),
        }
    }
}

/// Construct a feature file parser, built around step parsers
///
/// # Arguments
///
/// * `given`, `when`, `then` : User-defined parsers to parse and produce
/// `Step`s out of the text after `Given`, `When`, and `Then`, respectively.
pub fn feature<I, C, GP, WP, TP>(
    given: GP,
    when: WP,
    then: TP,
) -> impl Parser<Input = I, Output = Feature<C>>
where
    C: TestContext + 'static,
    GP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
    WP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
    TP: Parser<Input = I, Output = BoxedStep<C>> + Clone,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let blank_lines = || many1::<Vec<_>, _>(newline());

    let background = optional(
        (
            scenario("Background", given.clone(), when.clone(), then.clone()),
            blank_lines(),
        ).map(|t| TestCase::Background(t.0))
    );

    let test_cases = sep_by(
        scenario("Scenario", given, when, then).map(|s| TestCase::Scenario(s)),
        blank_lines());

    struct_parser! {
        Feature {
            _: optional(blank_lines()),
            _: string("Feature: "),
            name: until_eol(),
            comment: line_block(),
            _: blank_lines(),
            background: background,
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

        let (feat, remaining) = feature(
            given.map(|x| BoxedStep { val: Box::new(x) }),
            when.map(|x| BoxedStep { val: Box::new(x) }),
            then.map(|x| BoxedStep { val: Box::new(x) }),
        ).easy_parse(State::new(s))
            .unwrap();

        println!("End state: {:#?}", remaining);

        feat
    }

    #[test]
    fn test_parse() {
        let feat = do_parse(
            r"
Feature: my feature
one
two

Background:
Given G1

Scenario: One
Given G2
When W3
Then T4

Scenario: Two
Given G2
And G3
When W4
And W5
Then T6
And T7");

        assert_eq!(feat.name, "my feature".to_string());
        assert_eq!(feat.comment, "one\ntwo".to_string());
        assert!(feat.background.is_some());
        assert_eq!(feat.test_cases.len(), 2);
        assert_eq!(feat.test_cases[0].name(), Some("One".to_string()));
        assert_eq!(feat.test_cases[1].name(), Some("Two".to_string()));

        let results = feat.eval();

        assert_eq!(results[0].pass, true);
        assert_eq!(results[0].test_case_name, "One".to_string());
        assert_eq!(results[0].context.executed_steps, [1, 2, 3, 4]);

        assert_eq!(results[1].pass, true);
        assert_eq!(results[1].test_case_name, "Two".to_string());
        assert_eq!(results[1].context.executed_steps, [1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_parse_extra_whitespace() {
        let feat = do_parse(
            r"
Feature: my feature


Scenario: One
Given G1
When W1
Then T1


Scenario: Two
Given G2
When W2
Then T2
",
        );

        assert_eq!(feat.name, "my feature".to_string());
        assert_eq!(feat.comment, "".to_string());
        assert_eq!(feat.test_cases.len(), 2);
        assert_eq!(feat.test_cases[0].name(), Some("One".to_string()));
        assert_eq!(feat.test_cases[1].name(), Some("Two".to_string()));

        let results = feat.eval();

        assert_eq!(results[0].pass, true);
        assert_eq!(results[0].test_case_name, "One".to_string());
        assert_eq!(results[0].context.executed_steps, [1, 1, 1]);

        assert_eq!(results[1].pass, true);
        assert_eq!(results[1].test_case_name, "Two".to_string());
        assert_eq!(results[1].context.executed_steps, [2, 2, 2]);
    }
}
