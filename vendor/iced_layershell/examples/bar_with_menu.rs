use iced_layershell::widget::Column;
use iced_layershell::{
    Alignment, Anchor, Color, Element, Error, KeyboardInteractivity, Layer, LayerShellSettings,
    Length, SurfaceId, Task, application, button, container, destroy_layer_surface,
    new_layer_surface, row, text,
};

struct App {
    menu_id: Option<SurfaceId>,
    count: i32,
}

#[derive(Debug, Clone)]
enum Msg {
    OpenMenu,
    CloseMenu,
    Increment,
    Decrement,
}

fn boot() -> (App, Task<Msg>) {
    (
        App {
            menu_id: None,
            count: 0,
        },
        Task::none(),
    )
}

fn update(app: &mut App, msg: Msg) -> Task<Msg> {
    match msg {
        Msg::OpenMenu => {
            if app.menu_id.is_some() {
                return Task::none(); // Already open
            }
            let (id, task) = new_layer_surface(LayerShellSettings {
                anchor: Anchor::all(),
                layer: Layer::Overlay,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                namespace: "menu".into(),
                ..Default::default()
            });
            app.menu_id = Some(id);
            task
        }
        Msg::CloseMenu => {
            if let Some(id) = app.menu_id.take() {
                destroy_layer_surface(id)
            } else {
                Task::none()
            }
        }
        Msg::Increment => {
            app.count += 1;
            Task::none()
        }
        Msg::Decrement => {
            app.count -= 1;
            Task::none()
        }
    }
}

fn view(app: &App, id: SurfaceId) -> Element<'_, Msg> {
    if id == SurfaceId::MAIN {
        container(
            row![
                text(format!("Count: {}", app.count)).size(16),
                iced_layershell::widget::Space::new().width(Length::Fill),
                button("Menu").on_press(Msg::OpenMenu),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(4)
        .center_y(Length::Fill)
        .into()
    } else {
        // Menu overlay
        container(
            Column::new()
                .push(text("Menu Overlay").size(20))
                .push(
                    row![
                        button(" + ").on_press(Msg::Increment),
                        text(format!("{}", app.count)).size(24),
                        button(" - ").on_press(Msg::Decrement),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .push(button("Close Menu").on_press(Msg::CloseMenu))
                .spacing(10)
                .align_x(Alignment::Center),
        )
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
            ..Default::default()
        })
        .center(Length::Fill)
        .into()
    }
}

fn main() -> Result<(), Error> {
    application(boot, update, view)
        .layer_shell(LayerShellSettings {
            anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            layer: Layer::Top,
            exclusive_zone: 40,
            size: Some((0, 40)),
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "bar".into(),
            ..Default::default()
        })
        .run()
}
