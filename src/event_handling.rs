use crate::data::ProgramState;
use crate::key_poller::KeyPoller;
use std::rc::Rc;

pub async fn event_loop(window: Rc<pancurses::Window>) {
    let mut state = ProgramState{
        window: Rc::clone(&window),
        key_poller: KeyPoller::new(window),
    };

    let player_id = pasts::Loop::new(&mut state)
        .when(|s| &mut s.key_poller, on_key)
        .await;

    pancurses::endwin();
}

fn on_timer(state: &mut ProgramState, tick: usize) -> std::task::Poll<()> {
    std::task::Poll::Pending
}

fn on_key(state: &mut ProgramState, input: pancurses::Input) -> std::task::Poll<()> {
    print_at(&state.window, 4, 0, &format!("input: {:?}", input), true);

    match input {
        pancurses::Input::Character('q') => std::task::Poll::Ready(()),
        _ => std::task::Poll::Pending
    }
}

fn print_at(window: &pancurses::Window, y: i32, x: i32, s: &str, refresh: bool) {
    window.mv(y, x);
    window.clrtoeol();
    window.printw(s);
    if refresh { window.refresh(); }
}
