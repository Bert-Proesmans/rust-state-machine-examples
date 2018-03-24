extern crate automaton_test;

use std::marker::PhantomData;

use automaton_test::*;

fn new_machine() -> Machine<Wait<Start>> {
    Machine {
        state: PhantomData,
        transaction: Epsilon,
        storage: StorageService { 
        	tape: vec![]
        },
    }
}

fn main() {
    let start_state = new_machine();

    let input_state: Machine<Wait<Input>> = TransitionFrom::transition_from(start_state, Epsilon);

    let action_state: Machine<Action<Print>> = PushdownFrom::pushdown_from(input_state, PrintTransaction("Hello"));

    let deep_action_state: Machine<Action<Load>> = PushdownFrom::pushdown_from(action_state, Epsilon);

    let action_state: Machine<Action<Print>> = PullupFrom::pullup_from(deep_action_state);

    let input_state: Machine<Wait<Input>> = PullupFrom::pullup_from(action_state);

    let finished_state: Machine<Finished> = TransitionFrom::transition_from(input_state, Epsilon);

    println!("{:?}", finished_state);
}
