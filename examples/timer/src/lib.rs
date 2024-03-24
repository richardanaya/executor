use web::*;
use executor;
//use web::sleep;
#[no_mangle]
fn main() {
    executor::add_async(async {
        loop {
            set_inner_html(DOM_BODY, "⏰ tic");
            sleep(1000).await;
            set_inner_html(DOM_BODY, "⏰ tock");
            sleep(1000).await;
        }
    });
    while !executor::is_done() {
        executor::update();
    }
}
