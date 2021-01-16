//! This module takes care of drawing cards from a standard deck with optionnal Joker in it.
//!

use rand::prelude::SliceRandom;

#[derive(Debug, Copy, Clone)]
#[allow(missing_docs)]
/// Representation of the suits in a deck of cards
pub enum Suit {
    /// A joker has no suit, this variant is only used for a joker card
    None,
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Copy, Clone)]
/// A card
pub struct Card {
    /// Value of the card 1 to 10, Jack (11), Queen (12) and King (13), Joker (0)
    pub value: u32,
    /// Suit of the card, `Suit::None` if it's a Joker
    pub suit: Suit,
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self.value {
            0 => "0", // joker
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            10 => "10",
            11 => "J",
            12 => "Q",
            13 => "K",
            _ => unreachable!(),
        };

        let card = match self.suit {
            Suit::None => "🃏".to_string(),
            Suit::Clubs => format!("{}♣", value),
            Suit::Diamonds => format!("{}♦", value),
            Suit::Hearts => format!("{}♥", value),
            Suit::Spades => format!("{}♠", value),
        };

        write!(f, "{}", card)
    }
}

#[derive(Debug)]
/// Represent a standard deck of cards of 52 cards, with optionnal Jokers
///
/// `Deref` gives back the internal `Vec<Card>`
///
pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    /// Create a deck of cards, with provided number of jokers, and shuffle it
    pub fn new(nb_of_joker: usize) -> Self {
        Deck {
            cards: Deck::generate_deck(nb_of_joker),
        }
    }

    fn generate_deck(nb_of_joker: usize) -> Vec<Card> {
        let mut cards = Vec::with_capacity(52 + nb_of_joker);
        cards.append(&mut Deck::generate_one_suit(Suit::Clubs));
        cards.append(&mut Deck::generate_one_suit(Suit::Diamonds));
        cards.append(&mut Deck::generate_one_suit(Suit::Hearts));
        cards.append(&mut Deck::generate_one_suit(Suit::Spades));
        for _ in 0..nb_of_joker {
            cards.push(Card {
                value: 0,
                suit: Suit::None,
            });
        }
        let mut rng = rand::thread_rng();
        cards.shuffle(&mut rng);
        cards
    }

    fn generate_one_suit(suit: Suit) -> Vec<Card> {
        (1..14_u32).map(|value| Card { value, suit }).collect()
    }

    /// Draw the wanted number of cards, removing them from the Deck
    pub fn draw(&mut self, nb: usize) -> Vec<Card> {
        self.cards.drain(0..nb).collect()
    }

    /// Shuffle the remaining cards
    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        self.cards.shuffle(&mut rng);
    }

    /// Recreate the deck and shuffle it
    pub fn reset(&mut self, nb_of_joker: usize) {
        self.cards = Deck::generate_deck(nb_of_joker);
    }
}

impl std::ops::Deref for Deck {
    type Target = Vec<Card>;

    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

impl std::ops::DerefMut for Deck {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cards
    }
}
