use crate::data::ProgramState;
use crate::key_poller::KeyPoller;
use std::rc::Rc;

pub async fn event_loop(window: Rc<pancurses::Window>) {
    let mut state = ProgramState{
        counter: 0,
        window: Rc::clone(&window),
        key_poller: KeyPoller::new(window),
        timer: Box::pin(pasts::Past::new((), |()| async_std::task::sleep(std::time::Duration::from_secs(1)))),
    };

    let _ = pasts::Loop::new(&mut state)
        .when(|s| &mut s.key_poller, on_key)
        .when(|s| &mut s.timer, on_timer)
        .await;

    pancurses::endwin();
}

fn on_timer(state: &mut ProgramState, _: ()) -> std::task::Poll<()> {
    print_at(&state.window, 6, 0, &format!("tick: {}", state.counter), true);
    state.counter += 1;

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
