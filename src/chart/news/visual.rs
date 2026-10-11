use super::Message;
use iced::{
    Element,
    widget::{button, container, row, svg, text, tooltip},
};
use std::sync::OnceLock;

#[derive(Clone, Copy)]
#[repr(usize)]
pub(super) enum Icon {
    Refresh,
    Pause,
    Play,
    Filter,
    Reset,
    News,
    Link,
    Back,
    New,
    Clock,
    Shield,
    Globe,
    Chart,
    Network,
    Warning,
    Coin,
    Up,
    Down,
    Flat,
    Search,
    Check,
    Layers,
    Building,
    Scales,
}

pub(super) fn icon(which: Icon, size: u16) -> iced::widget::Svg<'static> {
    static HANDLES: OnceLock<Vec<svg::Handle>> = OnceLock::new();
    let handles = HANDLES.get_or_init(|| {
        vec![
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/refresh.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/pause.svg").as_slice(),
            ),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/play.svg").as_slice()),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/filter.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/reset.svg").as_slice(),
            ),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/news.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/link.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/back.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/new.svg").as_slice()),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/clock.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/shield.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/globe.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/chart.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/network.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/warning.svg").as_slice(),
            ),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/coin.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/up.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/down.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/flat.svg").as_slice()),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/search.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/check.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/layers.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/building.svg").as_slice(),
            ),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/scales.svg").as_slice(),
            ),
        ]
    });
    svg(handles[which as usize].clone())
        .width(f32::from(size))
        .height(f32::from(size))
        .style(|theme: &iced::Theme, _| svg::Style {
            color: Some(theme.palette().background.base.text),
        })
}

pub(super) fn asset(symbol: &str, size: u16) -> Element<'static, Message> {
    static HANDLES: OnceLock<Vec<svg::Handle>> = OnceLock::new();
    let index = match symbol {
        "BTC" => 0,
        "ETH" => 1,
        "SOL" => 2,
        "LINK" => 3,
        "ZEC" => 4,
        "XMR" => 5,
        "LTC" => 6,
        _ => return icon(Icon::Coin, size).into(),
    };
    let handles = HANDLES.get_or_init(|| {
        vec![
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/btc.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/eth.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/sol.svg").as_slice()),
            svg::Handle::from_memory(
                include_bytes!("../../../assets/ui/news/chainlink.svg").as_slice(),
            ),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/zec.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/xmr.svg").as_slice()),
            svg::Handle::from_memory(include_bytes!("../../../assets/ui/news/ltc.svg").as_slice()),
        ]
    });
    svg(handles[index].clone())
        .width(f32::from(size))
        .height(f32::from(size))
        .into()
}

pub(super) fn category(name: &str) -> Icon {
    match name {
        "Security" => Icon::Shield,
        "Systemic risk" => Icon::Warning,
        "Macro" => Icon::Globe,
        "Regulation" => Icon::Scales,
        "Institutional" => Icon::Building,
        "Supply / listing" => Icon::Layers,
        "Network" => Icon::Network,
        _ => Icon::Chart,
    }
}

pub(super) fn action(
    which: Icon,
    label: &'static str,
    message: Option<Message>,
    active: bool,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let picture = icon(which, 18).style(move |theme: &iced::Theme, _| svg::Style {
        color: Some(if active {
            theme.palette().warning.base.color
        } else {
            theme
                .palette()
                .background
                .base
                .text
                .scale_alpha(if enabled { 1.0 } else { 0.35 })
        }),
    });
    tooltip(
        button(container(picture).center(32))
            .padding(0)
            .on_press_maybe(message)
            .style(move |theme, status| crate::style::button::transparent(theme, status, active)),
        container(text(label).size(11))
            .padding(8)
            .style(crate::style::tooltip),
        tooltip::Position::Bottom,
    )
    .delay(crate::widget::DEFAULT_TOOLTIP_DELAY)
    .into()
}

pub(super) fn named_action(
    which: Icon,
    label: String,
    message: Message,
) -> Element<'static, Message> {
    button(
        row![icon(which, 16), text(label).size(11)]
            .spacing(6)
            .align_y(iced::Alignment::Center),
    )
    .padding([5, 7])
    .on_press(message)
    .style(|theme, status| crate::style::button::transparent(theme, status, false))
    .into()
}
