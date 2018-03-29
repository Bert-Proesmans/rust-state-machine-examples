extern crate automaton_test;

use std::env;
use std::marker::PhantomData;

use automaton_test::service::StackStorage;
use automaton_test::state::*;
use automaton_test::stm::*;
use automaton_test::transaction::{Epsilon, PrintTransaction};
use automaton_test::*;

fn new_machine() -> Machine<Wait<Start>> {
    Machine {
        state: PhantomData,
        transaction: Epsilon,
        storage: StackStorage { tape: vec![] },
    }
}

fn main() {
    // DBG; This will enable Failure to print out full backtraces.
    // env::set_var("RUST_BACKTRACE", "1");

    let start_state = new_machine();

    // DBG; The following syntax can/will be made simpler by implementing the TransitionInto-
    // counterpart of TransitionFrom.
    let input_state: Machine<Wait<Input>> = start_state.transition(Epsilon);

    let action_state: Machine<Action<Print>> = input_state.pushdown(PrintTransaction("Hello"));

    println!("Printing transaction: {:?}", action_state.transaction);

    let deep_action_state: Machine<Action<Load>> = action_state.pushdown(Epsilon);

    let action_state: Machine<Action<Print>> =
        deep_action_state.pullup().expect("Transition Error");

    println!("Validate transaction: {:?}", action_state.transaction);

    let input_state: Machine<Wait<Input>> = action_state.pullup().expect("Transition Error");

    let finished_state: Machine<Finished> = input_state.transition(Epsilon);

    println!("{:?}", finished_state);
}
