use iced_layershell::{
    Alignment, Anchor, Color, Element, Error, KeyboardInteractivity, Layer, LayerShellSettings,
    Length, SurfaceId, Task, application, button, clipboard, container, row, text, text_input,
};

struct App {
    count: i32,
    input_value: String,
    secret: String,
}

#[derive(Debug, Clone)]
enum Msg {
    Increment,
    Decrement,
    InputChanged(String),
    CopySecret,
}

fn boot() -> (App, Task<Msg>) {
    (
        App {
            count: 0,
            input_value: String::new(),
            secret: "ABC-123-SECRET".into(),
        },
        Task::none(),
    )
}

fn update(app: &mut App, msg: Msg) -> Task<Msg> {
    match msg {
        Msg::Increment => {
            app.count += 1;
            Task::none()
        }
        Msg::Decrement => {
            app.count -= 1;
            Task::none()
        }
        Msg::InputChanged(value) => {
            app.input_value = value;
            Task::none()
        }
        Msg::CopySecret => Task::from(clipboard::write(app.secret.clone())),
    }
}

fn view(app: &App, _id: SurfaceId) -> Element<'_, Msg> {
    let btn_style = |_theme: &iced_layershell::Theme, status: button::Status| match status {
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgb(0.4, 0.4, 0.7).into()),
            text_color: Color::WHITE,
            ..Default::default()
        },
        _ => button::Style {
            background: Some(Color::from_rgb(0.3, 0.3, 0.5).into()),
            text_color: Color::WHITE,
            ..Default::default()
        },
    };

    container(
        row![
            button(text(" - ").size(14))
                .on_press(Msg::Decrement)
                .style(btn_style),
            text(format!(" {} ", app.count)).size(16),
            button(text(" + ").size(14))
                .on_press(Msg::Increment)
                .style(btn_style),
            text_input("Type here...", &app.input_value)
                .on_input(Msg::InputChanged)
                .width(200),
            text(format!("Secret: {}", app.secret)).size(12),
            button(text("Copy").size(12))
                .on_press(Msg::CopySecret)
                .style(btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(4)
    .center_y(Length::Fill)
    .into()
}

fn main() -> Result<(), Error> {
    application(boot, update, view)
        .layer_shell(LayerShellSettings {
            anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            layer: Layer::Top,
            exclusive_zone: 40,
            size: Some((0, 40)),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            namespace: "simple_bar".into(),
            ..Default::default()
        })
        .run()
}
