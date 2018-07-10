/// A test context is used to pass state between different steps of a test case.
pub trait TestContext {
    fn new() -> Self;
}


pub enum TestCase<C: TestContext> {
    Background(Scenario<C>),
    Scenario(Scenario<C>),
}

impl<C: TestContext> TestCase<C> {
    pub fn name(&self) -> Option<String> {
        match self {
            TestCase::Background(s) => s.name.clone(),
            TestCase::Scenario(s) => s.name.clone()
        }
    }


    pub fn eval(&self, context: C) -> TestResult<C> {
        match self {
            TestCase::Background(s) => s.eval(context),
            TestCase::Scenario(s) => s.eval(context)
        }
    }
}

pub struct TestResult<C: TestContext> {
    pub test_case_name: String,
    pub pass: bool,
    pub context: C
}

/// A feature is a collection of test cases.
pub struct Feature<C: TestContext> {
    pub name: String,
    pub comment: String,
    pub background: Option<TestCase<C>>,
    pub test_cases: Vec<TestCase<C>>,
}

impl<C: TestContext> Feature<C> {
    pub fn eval(&self) -> Vec<TestResult<C>> {

        let mut results = vec![];
        for tc in self.test_cases.iter() {
            let mut context = C::new();

            if let Some(TestCase::Background(ref bg)) = self.background {
                match bg.eval(context) {
                    mut r @ TestResult { pass: false, ..} => {
                        r.test_case_name = "<Background>".to_string();
                        results.push(r);
                        continue;
                    },
                    TestResult { pass: true, context: c, ..} => {
                        context = c;
                    }
                }
            }

            results.push(tc.eval(context));
        }

        results
    }
}

pub struct Scenario<TC: TestContext> {
    pub name: Option<String>,
    pub steps: Vec<Box<Step<TC>>>,
}

impl<C: TestContext> Scenario<C> {
    /// Execute a scenario by running each step in order, with mutable access to
    /// the context.
    pub fn eval(&self, mut context: C) -> TestResult<C> {
        for s in self.steps.iter() {
            if !s.eval(&mut context) {
                return TestResult {
                    test_case_name: match self.name.as_ref() {
                        Some(s) => s.clone(),
                        None => "".to_string()
                    },
                    pass: false,
                    context: context
                };
            }
        }

        TestResult {
            test_case_name: match self.name.as_ref() {
                Some(s) => s.clone(),
                None => "".to_string()
            },
            pass: true,
            context: context
        }
    }
}

/// A specific step which makes up a scenario. Users should create their own
/// implementations of this trait, which are returned by their step parsers.
pub trait Step<C: TestContext> {
    fn eval(&self, &mut C) -> bool;
}

