#[cfg(all(tokio_unstable, feature = "tracing"))]
mod tests {
    use std::rc::Rc;
    use tokio::{
        task::{Builder, LocalSet},
        test,
    };

    #[test]
    async fn spawn_with_name() {
        let result = Builder::new()
            .name("name")
            .spawn(async { "task executed" })
            .unwrap()
            .await;

        assert_eq!(result.unwrap(), "task executed");
    }

    #[test]
    async fn spawn_blocking_with_name() {
        let result = Builder::new()
            .name("name")
            .spawn_blocking(|| "task executed")
            .unwrap()
            .await;

        assert_eq!(result.unwrap(), "task executed");
    }

    #[test]
    async fn spawn_local_with_name() {
        let unsend_data = Rc::new("task executed");
        let result = LocalSet::new()
            .run_until(async move {
                Builder::new()
                    .name("name")
                    .spawn_local(async move { unsend_data })
                    .unwrap()
                    .await
            })
            .await;

        assert_eq!(*result.unwrap(), "task executed");
    }

    #[test]
    async fn spawn_without_name() {
        let result = Builder::new()
            .spawn(async { "task executed" })
            .unwrap()
            .await;

        assert_eq!(result.unwrap(), "task executed");
    }

    #[test]
    async fn spawn_blocking_without_name() {
        let result = Builder::new()
            .spawn_blocking(|| "task executed")
            .unwrap()
            .await;

        assert_eq!(result.unwrap(), "task executed");
    }

    #[test]
    async fn spawn_local_without_name() {
        let unsend_data = Rc::new("task executed");
        let result = LocalSet::new()
            .run_until(async move {
                Builder::new()
                    .spawn_local(async move { unsend_data })
                    .unwrap()
                    .await
            })
            .await;

        assert_eq!(*result.unwrap(), "task executed");
    }

    use futures::future;
    use std::future::Future;
    use tracing::subscriber::{set_default, with_default};
    use tracing_attributes::instrument;
    use tracing_mock::{
        field,
        span::{self, NewSpan},
        subscriber,
    };

    #[instrument(fields(foo = "bar", dsa = true, num = 1))]
    fn fn_no_param() {}

    #[test]
    async fn fields() {
        let span = span::mock().with_field(
            field::mock("foo")
                .with_value(&"bar")
                .and(field::mock("dsa").with_value(&true))
                .and(field::mock("num").with_value(&1_i64))
                .only(),
        );
        run_test(span, || {
            fn_no_param();
        });
    }

    #[test]
    async fn fields_set_default() {
        let span = span::mock().with_field(
            field::mock("foo")
                .with_value(&"bar")
                .and(field::mock("dsa").with_value(&true))
                .and(field::mock("num").with_value(&1_i64))
                .only(),
        );
        let (subscriber, handle) = subscriber::mock()
            .new_span(span)
            .enter(span::mock())
            .exit(span::mock())
            .done()
            .run_with_handle();

        let guard = set_default(subscriber);

        fn_no_param();

        drop(guard);
        handle.assert_finished();
    }

    #[test]
    async fn manual_span_enter() {
        let kind = "local";
        let span = span::mock().with_field(
            field::mock("kind")
                .with_value(&tracing::field::display(kind))
                // .and(field::mock("dsa").with_value(&true))
                // .and(field::mock("num").with_value(&1_i64))
                .only(),
        );
        let (subscriber, handle) = subscriber::mock()
            .new_span(span)
            .enter(span::mock())
            .exit(span::mock())
            .done()
            .run_with_handle();

        let guard = set_default(subscriber);

        // Start test.
        let span = tracing::trace_span!(target: "tokio::task", "runtime.spawn", %kind);
        let span_guard = span.enter();

        drop(span_guard);
        drop(span);
        // End test.

        drop(guard);
        handle.assert_finished();
    }

    #[test]
    async fn spawn_local_location() {
        let this_file = file!();
        let span = span::mock().with_field(
            field::mock("kind")
                .with_value(&tracing::field::display("local"))
                .and(field::mock("task.name").with_value(&tracing::field::display("")))
                .and(field::mock("loc.file").with_value(&this_file))
                .and(field::mock("task.id"))
                .and(field::mock("loc.line"))
                .and(field::mock("loc.col"))
                .only(),
        );
        let (subscriber, handle) = subscriber::mock()
            .new_span(span)
            .enter(span::mock())
            .exit(span::mock())
            .done()
            .run_with_handle();

        let guard = set_default(subscriber);

        // Start test.
        let _result = LocalSet::new()
            .run_until(async { Builder::new().spawn_local(future::ready(())).unwrap().await })
            .await;
        // End test.

        drop(guard);
        handle.assert_finished();
    }

    #[test]
    async fn spawn_local_location_run_test() {
        let span = mock_task_span("local", file!());

        run_test_async(span, async {
            let _result = LocalSet::new()
                .run_until(async { Builder::new().spawn_local(future::ready(())).unwrap().await })
                .await;
        })
        .await;
    }

    fn mock_task_span(kind: &str, loc_file: &str) -> NewSpan {
        span::mock().with_field(
            field::mock("kind")
                .with_value(&tracing::field::display(kind))
                .and(field::mock("task.name").with_value(&tracing::field::display("")))
                .and(field::mock("loc.file").with_value(&loc_file))
                .and(field::mock("task.id"))
                .and(field::mock("loc.line"))
                .and(field::mock("loc.col"))
                .only(),
        )
    }

    async fn run_test_async<T>(span: NewSpan, future: T)
    where
        T: Future,
    {
        let (subscriber, handle) = subscriber::mock()
            .new_span(span)
            .enter(span::mock())
            .exit(span::mock())
            .done()
            .run_with_handle();

        let guard = set_default(subscriber);
        future.await;
        drop(guard);

        handle.assert_finished();
    }

    fn run_test<F: FnOnce() -> T, T>(span: NewSpan, fun: F) {
        let (subscriber, handle) = subscriber::mock()
            .new_span(span)
            .enter(span::mock())
            .exit(span::mock())
            .done()
            .run_with_handle();

        with_default(subscriber, fun);
        handle.assert_finished();
    }
}
