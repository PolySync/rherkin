use combine::ParseError;
use combine::Parser;
use combine::Stream;

use combine::char::{newline, string};
use combine::sep_by;
use parse_utils::{line_block, until_eol};

/// A test context is used to pass state between different steps of a test case.
/// It may also be initialized at the feature level via a Background (TODO)
pub trait TestContext {
    fn new() -> Self;
}

/// A test case is a generalization of a Scenario; might also be a PropTest.
pub trait TestCase<C: TestContext> {
    fn name(&self) -> String;
    fn eval(&self, C) -> (bool, C);
}

/// A feature is a collection of tests.
// TODO: Implement background
pub struct Feature<C: TestContext> {
    pub name: String,
    pub comment: String,
    pub test_cases: Vec<Box<TestCase<C>>>,
}

impl<C: TestContext> Feature<C> {
    pub fn eval(&self) -> (bool, C) {
        let mut context = C::new();

        for tc in self.test_cases.iter() {
            let (pass, context_) = tc.eval(context);
            context = context_;

            if !pass {
                return (false, context);
            }
        }

        (true, context)
    }
}

/// The output of a test case parser. Ideally this would just be Box<TestCase>
/// but there's some subtle issue to do with using trait objects in an
/// associated type, perhaps in concert with the 'impl trait' feature, that
/// keeps it from working. Wrapping it in a struct is a workaround.
pub struct BoxedTestCase<C: TestContext> {
    pub val: Box<TestCase<C>>,
}

/// Construct a feature file parser, built around a test-case parser.
pub fn parser<I, C, TP>(test_case_parser: TP) -> impl Parser<Input = I, Output = Feature<C>>
where
    C: TestContext,
    TP: Parser<Input = I, Output = BoxedTestCase<C>>,
    I: Stream<Item = char>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let test_cases =
        sep_by(test_case_parser, newline()).map(|results: Vec<BoxedTestCase<C>>| {
            results.into_iter().map(|result| result.val).collect()
        });

    struct_parser! {
        Feature {
            _: string("Feature: "),
            name: until_eol(),
            comment: line_block(),
            _: newline(),
            test_cases: test_cases
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use combine::stream::easy;
    use combine::stream::state::State;

    /// The sample test case just records each test in this context as it runs
    struct SampleTestContext {
        executed_cases: Vec<String>,
        executed_contents: Vec<String>,
    }

    impl TestContext for SampleTestContext {
        fn new() -> SampleTestContext {
            SampleTestContext {
                executed_cases: vec![],
                executed_contents: vec![],
            }
        }
    }

    struct SampleTestCase {
        name: String,
        content: String,
    }

    impl TestCase<SampleTestContext> for SampleTestCase {
        fn name(&self) -> String {
            self.name.clone()
        }

        fn eval(&self, mut context: SampleTestContext) -> (bool, SampleTestContext) {
            context.executed_cases.push(self.name.clone());
            context.executed_contents.push(self.content.clone());
            (true, context)
        }
    }

    fn sample_test_case_parser<'a, I>() -> impl Parser<Input = I, Output = BoxedTestCase<SampleTestContext>>
    where
        I: Stream<Item = char>,
        I::Error: ParseError<I::Item, I::Range, I::Position>,
    {
        let p = struct_parser!{
            SampleTestCase {
                _: string("Sample Test Case: "),
                name: until_eol(),
                content: line_block(),
            }
        };

        p.map(|stc| BoxedTestCase { val: Box::new(stc) })
    }

    #[test]
    fn feature() {
        let s = "Feature: my feature\n\
                 comment line one\n\
                 comment line two\n\
                 \n\
                 Sample Test Case: first\n\
                 Content one\n\
                 \n\
                 Sample Test Case: second\n\
                 Content two\n";

        let (feat, remaining) = parser(sample_test_case_parser())
            .easy_parse(State::new(s))
            .unwrap();
        println!("End state: {:#?}", remaining);

        assert_eq!(feat.name, "my feature".to_string());
        assert_eq!(feat.comment, "comment line one\ncomment line two".to_string());
        assert_eq!(feat.test_cases.len(), 2);
        assert_eq!(feat.test_cases[0].name(), "first".clone());
        assert_eq!(feat.test_cases[1].name(), "second".clone());

        let (pass, ctx) = feat.eval();
        assert!(pass);
        assert_eq!(ctx.executed_cases,
                   vec!("first".to_string(), "second".to_string()));
        assert_eq!(ctx.executed_contents,
                   vec!("Content one".to_string(), "Content two".to_string()));
    }
}
