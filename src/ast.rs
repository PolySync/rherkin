/// A test context is used to pass state between different steps of a test case.
/// It may also be initialized at the feature level via a Background (TODO)
pub trait TestContext {
    fn new() -> Self;
}


pub enum TestCase<C: TestContext> {
    Background(Scenario<C>),
    Scenario(Scenario<C>),
}

impl<C: TestContext> TestCase<C> {
    pub fn name(&self) -> String {
        match self {
            TestCase::Background(s) => s.name.clone(),
            TestCase::Scenario(s) => s.name.clone()
        }
    }


    pub fn eval(&self, context: &mut C) -> bool {
        match self {
            TestCase::Background(s) => s.eval(context),
            TestCase::Scenario(s) => s.eval(context)
        }
    }
}

/// A feature is a collection of test cases.
pub struct Feature<C: TestContext> {
    pub name: String,
    pub comment: String,
    pub test_cases: Vec<TestCase<C>>,
}

impl<C: TestContext> Feature<C> {
    pub fn eval(&self) -> (bool, C) {
        let mut context = C::new();

        for tc in self.test_cases.iter() {
            let pass = tc.eval(&mut context);

            if !pass {
                return (false, context);
            }
        }

        (true, context)
    }
}

pub struct Scenario<TC: TestContext> {
    pub name: String,
    pub steps: Vec<Box<Step<TC>>>,
}

impl<C: TestContext> Scenario<C> {
    /// Execute a scenario by running each step in order, with mutable access to
    /// the context.
    pub fn eval(&self, context: &mut C) -> bool {
        for s in self.steps.iter() {
            if !s.eval(context) {
                return false;
            }
        }

        true
    }
}

/// A specific step which makes up a scenario. Users should create their own
/// implementations of this trait, which are returned by their step parsers.
pub trait Step<C: TestContext> {
    fn eval(&self, &mut C) -> bool;
}

