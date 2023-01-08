use tui::widgets::ListState;

#[derive(Clone)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    selected: usize,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
            selected: 0,
        }
    }

    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
            selected: 0,
        }
    }

    pub fn select_first(&mut self) {
        if self.items.len() > 0 {
            self.selected = 0;
            self.state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if self.items.len() > 0 {
            self.selected = self.items.len() - 1;
            self.state.select(Some(self.items.len() - 1));
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.selected = i;
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.selected = i;
        self.state.select(Some(i));
    }

    pub fn get_selected_id(&mut self) -> usize {
        self.selected
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
