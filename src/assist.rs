use rand::Rng;
use chrono::prelude::*;

pub fn generate_paste_id() -> String {

    let mut rng = rand::thread_rng();
    let millis = Utc::now().timestamp_millis();
    let random_number = rng.gen_range(10_000_000..999_999_999);

    let untouched = format!("{}:{}:anabin", millis, random_number);
    return format!("{:x}", md5::compute(untouched));
}

#[macro_export] macro_rules! whoops {
    () => {
        return HttpResponse::InternalServerError().body("whoops ;p");
    };
}
