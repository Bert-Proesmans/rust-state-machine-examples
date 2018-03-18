extern crate automaton_test;

use std::marker::PhantomData;

use automaton_test::*;

fn new_machine() -> Machine<Wait<Start>> {
    Machine {
        state: PhantomData,
        transaction: (),
        storage: StorageService {},
    }
}

fn main() {
    let start_state = new_machine();

    let input_state: Machine<Wait<Input>> = TransitionFrom::transition_from(start_state, ());

    let finished_state: Machine<Finished> = TransitionFrom::transition_from(input_state, ());

    println!("{:?}", finished_state);
}
