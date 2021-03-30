use web::*;

#[no_mangle]
fn main() {
    executor::run(async {
        loop {
            set_inner_html(DOM_BODY, "⏰ tic");
            sleep(1000).await;
            set_inner_html(DOM_BODY, "⏰ tock");
            sleep(1000).await;
        }
    });
}
