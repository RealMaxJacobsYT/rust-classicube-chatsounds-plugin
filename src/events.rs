use crate::{chat::CHAT, chatsounds::CHATSOUNDS, printer::print, thread};
use classicube_sys::{
  ChatEvents, Event_RegisterChat, Event_RegisterInput, Event_RegisterInt, Event_UnregisterChat,
  Event_UnregisterInput, Event_UnregisterInt, InputEvents, Key_, Key__KEY_TAB, MsgType,
  MsgType_MSG_TYPE_NORMAL,
};
use rand::seq::SliceRandom;
use std::{
  cell::RefCell,
  convert::TryInto,
  os::raw::{c_int, c_void},
  ptr,
};

fn handle_chat_message<S: Into<String>>(full_msg: S) {
  let mut full_msg = full_msg.into();

  if let Some(pos) = full_msg.rfind("&f") {
    let msg = full_msg.split_off(pos + 2);

    thread::spawn("chatsounds handle message", move || {
      if let Some(chatsounds) = CHATSOUNDS.lock().as_mut() {
        let msg = msg.trim();

        if msg.to_lowercase() == "sh" {
          chatsounds.stop_all();
          return;
        }

        if let Some(sounds) = chatsounds.get(msg) {
          let mut rng = rand::thread_rng();

          if let Some(sound) = sounds.choose(&mut rng).cloned() {
            chatsounds.play(&sound);
          }
        }
      }
    });
  }
}

thread_local! {
  static CHAT_LAST: RefCell<Option<String>> = RefCell::new(None);
  pub static TYPE: RefCell<Option<String>> = RefCell::new(None);
}

extern "C" fn on_chat_received(
  _obj: *mut c_void,
  full_msg: *const classicube_sys::String,
  msg_type: c_int,
) {
  let msg_type: MsgType = msg_type.try_into().unwrap();

  if msg_type != MsgType_MSG_TYPE_NORMAL {
    return;
  }
  let full_msg = if full_msg.is_null() {
    return;
  } else {
    unsafe { *full_msg }
  };

  let mut full_msg = full_msg.to_string();

  CHAT_LAST.with(|chat_last| {
    let mut chat_last = chat_last.borrow_mut();

    if !full_msg.starts_with("> &f") {
      *chat_last = Some(full_msg.clone());
    } else if let Some(chat_last) = &*chat_last {
      // we're a continue message
      full_msg = full_msg.split_off(4); // skip "> &f"

      // most likely there's a space
      // the server trims the first line :(
      // TODO try both messages? with and without the space?
      full_msg = format!("{} {}", chat_last, full_msg);
    }
  });

  handle_chat_message(&full_msg);
}

extern "C" fn on_key_down(_obj: *mut c_void, key: c_int, repeat: u8) {
  let key: Key_ = key.try_into().unwrap();

  CHAT.with(|chat| {
    let mut chat = chat.borrow_mut();

    chat.handle_key_down(key, repeat != 0);

    if chat.is_open() && key == Key__KEY_TAB {
      let text = chat.get_text();
      let text = text.trim();

      if !text.is_empty() {
        if let Some(chatsounds) = CHATSOUNDS.lock().as_mut() {
          let results = chatsounds.search(text);

          let mut ag = None;
          for &result in results.iter().take(3).rev() {
            print(result);
            ag = Some(result.to_string());
          }

          if let Some(text) = ag {
            TYPE.with(|t| {
              *t.borrow_mut() = Some(text);
            });
          }
        }
      }
    }
  });
}

extern "C" fn on_key_press(_obj: *mut c_void, key: c_int) {
  CHAT.with(|chat| {
    let mut chat = chat.borrow_mut();

    chat.handle_key_press(key);
  });
}

pub fn load() {
  unsafe {
    Event_RegisterChat(
      &mut ChatEvents.ChatReceived,
      ptr::null_mut(),
      Some(on_chat_received),
    );

    Event_RegisterInput(&mut InputEvents.Down, ptr::null_mut(), Some(on_key_down));
    Event_RegisterInt(&mut InputEvents.Press, ptr::null_mut(), Some(on_key_press));
  }
}

pub fn unload() {
  unsafe {
    Event_UnregisterChat(
      &mut ChatEvents.ChatReceived,
      ptr::null_mut(),
      Some(on_chat_received),
    );

    Event_UnregisterInput(&mut InputEvents.Down, ptr::null_mut(), Some(on_key_down));
    Event_UnregisterInt(&mut InputEvents.Press, ptr::null_mut(), Some(on_key_press));
  }
}
