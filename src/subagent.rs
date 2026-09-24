/// A child agent receives only an explicit task and copied context, never its parent's state.
pub trait ChildAgent {
    type Output;
    type Error;
    fn run(&self, task: &str, context: &[String]) -> Result<Self::Output, Self::Error>;
}

pub struct Subagent<A> {
    child: A,
    context: Vec<String>,
}
impl<A> Subagent<A> {
    pub fn new(child: A, context: impl IntoIterator<Item = String>) -> Self {
        Self {
            child,
            context: context.into_iter().collect(),
        }
    }
}
impl<A: ChildAgent> Subagent<A> {
    pub fn delegate(&self, task: &str) -> Result<A::Output, A::Error> {
        self.child.run(task, &self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Echo;
    impl ChildAgent for Echo {
        type Output = String;
        type Error = ();
        fn run(&self, task: &str, context: &[String]) -> Result<String, ()> {
            Ok(format!("{task}:{}", context.join(",")))
        }
    }
    #[test]
    fn delegates_with_a_copied_explicit_context_boundary() {
        let child = Subagent::new(Echo, ["fact".to_owned()]);
        assert_eq!(child.delegate("summarize"), Ok("summarize:fact".into()));
    }
}
