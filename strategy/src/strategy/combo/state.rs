use models::{fvg::FVG, swing::Swing, Candle};
use statig::{state_machine, Response};
use types::Timeframe;

pub enum Event {
    Fvg(FVG),
    Swing(Swing),
    CandleClose(Candle),
}

#[state_machine(initial = "State::idle()")]
impl super::Combo {
    #[state]
    fn idle(&mut self, event: &Event) -> Response<State> {
        let res = match event {
            Event::Fvg(x) => {
                if *x.timeframe() == Timeframe::Day(1).to_string() {
                    self.v1 = Some(x.clone());
                    println!("idle -> v1: {x:#?}");
                    Response::Transition(State::v1())
                } else {
                    Response::Handled
                }
            },
            _ => Response::Super,
        };
        return res;
    }

    #[state]
    fn v1(&mut self, event: &Event) -> Response<State> {
        let res = match event {
            Event::CandleClose(x) => {
                if *x.timeframe() == Timeframe::Hour(4).to_string() {
                    let v1 = self.v1.as_ref().expect("v1 should be set in State::v1()");
                    if v1.flow() == "bull" && x.low() <= v1.high() {
                    println!("v1 -> v1_test: {x:#?}");
                        Response::Transition(State::v1_test())
                    } else if v1.flow() == "bear" && x.high() >= v1.low() {
                    println!("v1 -> v1_test: {x:#?}");
                        Response::Transition(State::v1_test())
                    } else {
                        Response::Handled
                    }
                } else {
                    Response::Handled
                }
            },
            Event::Fvg(x) => {
                if *x.timeframe() == Timeframe::Day(1).to_string() {
                    self.v1 = Some(x.clone());
                    println!("new v1: {x:#?}");
                }
                Response::Handled
            },
            _ => Response::Super,
        };
        return res;
    }

    #[state]
    fn v1_test(&mut self, event: &Event) -> Response<State> {
        let res = match event {
            Event::Fvg(x) => {
                let v1 = self.v1.as_ref().expect("v1 should be set in State::v1_test()");
                if *x.timeframe() == Timeframe::Hour(4).to_string() && x.flow() == v1.flow() {
                    self.v2 = Some(x.clone());
                    println!("v1_test -> v2: {x:#?}");
                    Response::Transition(State::v2())
                } else {
                    Response::Handled
                }
            },
            _ => Response::Super,
        };
        return res;
    }

    #[state]
    fn v2(&mut self, event: &Event) -> Response<State> {
        let res = match event {
            Event::CandleClose(x) => {
                if *x.timeframe() == Timeframe::Hour(4).to_string() {
                    self.v1.take();
                    self.v2.take();
                    self.v3.take();
                    self.v4.take();
                    println!("v2 -> idle");
                    Response::Transition(State::idle())
                } else {
                    Response::Handled
                }
            },
            _ => Response::Super,
        };
        return res;
    }
}
