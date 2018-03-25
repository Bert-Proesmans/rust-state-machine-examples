extern crate automaton_test;

use std::marker::PhantomData;

use automaton_test::*;
use automaton_test::state::*;
use automaton_test::service::StackStorage;
use automaton_test::stm::*;
use automaton_test::transaction::{Epsilon, PrintTransaction};

fn new_machine() -> Machine<Wait<Start>> {
    Machine {
        state: PhantomData,
        transaction: Epsilon,
        storage: StackStorage { tape: vec![] },
    }
}

fn main() {
    let start_state = new_machine();

    // DBG; The following syntax can/will be made simpler by implementing the TransitionInto-
    // counterpart of TransitionFrom.
    let input_state: Machine<Wait<Input>> = TransitionFrom::transition_from(start_state, Epsilon);

    let action_state: Machine<Action<Print>> =
        PushdownFrom::pushdown_from(input_state, PrintTransaction("Hello"));

    println!("Printing transaction: {:?}", action_state.transaction);

    let deep_action_state: Machine<Action<Load>> =
        PushdownFrom::pushdown_from(action_state, Epsilon);

    let action_state: Machine<Action<Print>> =
        PullupFrom::pullup_from(deep_action_state).expect("Transition Error");

    println!("Validate transaction: {:?}", action_state.transaction);

    let input_state: Machine<Wait<Input>> =
        PullupFrom::pullup_from(action_state).expect("Transition Error");

    let finished_state: Machine<Finished> = TransitionFrom::transition_from(input_state, Epsilon);

    println!("{:?}", finished_state);
}
