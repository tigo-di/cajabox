//! Arrastar arquivos do manabox para outro programa (gerenciador de arquivos, navegador),
//! via GTK. Oferece copiar e mover; quem decide é o programa que recebe (ex.: no Nautilus,
//! mesmo disco = mover, outro disco = copiar; Ctrl/Shift trocam).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::gdk::DragAction;
use gtk::glib::{ObjectExt, SignalHandlerId};
use gtk::prelude::{DragContextExtManual, WidgetExt};
use gtk::{ApplicationWindow, TargetList};

use crate::clipboard::path_to_uri;

/// Inicia o arrasto de `paths` a partir da janela (com o botão do mouse pressionado).
/// Precisa rodar na thread principal. `on_end` é chamado quando o arrasto termina (soltou ou cancelou).
pub fn start(window: &ApplicationWindow, paths: &[PathBuf], on_end: impl Fn() + 'static) -> Result<(), String> {
    let uris: Vec<String> = paths.iter().map(|p| path_to_uri(p)).collect();
    let handlers: Rc<RefCell<Vec<SignalHandlerId>>> = Rc::default();

    let data_get = window.connect_drag_data_get(move |_, _, data, _, _| {
        let uris: Vec<&str> = uris.iter().map(String::as_str).collect();
        data.set_uris(&uris);
    });
    let end_handlers = handlers.clone();
    let drag_end = window.connect_drag_end(move |window, _| {
        for id in end_handlers.borrow_mut().drain(..) {
            window.disconnect(id); // também remove este próprio handler
        }
        on_end();
    });
    handlers.borrow_mut().extend([data_get, drag_end]);

    let targets = TargetList::new(&[]);
    targets.add_uri_targets(0);
    match window.drag_begin_with_coordinates(&targets, DragAction::COPY | DragAction::MOVE, 1, None, -1, -1) {
        Some(context) => {
            let icon = if paths.len() > 1 { "emblem-documents" } else { "text-x-generic" };
            context.drag_set_icon_name(icon, 0, 0);
            Ok(())
        }
        None => {
            for id in handlers.borrow_mut().drain(..) {
                window.disconnect(id);
            }
            Err(crate::tr!("Não foi possível iniciar o arrasto", "Could not start dragging"))
        }
    }
}
