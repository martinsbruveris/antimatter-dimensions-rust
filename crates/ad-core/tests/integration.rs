use ad_core::GameState;
use break_infinity::Decimal;

#[test]
fn test_new_game_starts_with_10_antimatter() {
    let game = GameState::new();
    assert_eq!(game.antimatter, Decimal::from_float(10.0));
}

#[test]
fn test_buy_ad1_spends_antimatter() {
    let mut game = GameState::new();
    assert!(game.buy_ad1());
    // Spent 10, left with 0
    assert_eq!(game.antimatter, Decimal::from_float(0.0));
    assert_eq!(game.ad1.bought, 1);
}

#[test]
fn test_cannot_buy_ad1_without_antimatter() {
    let mut game = GameState::new();
    game.buy_ad1(); // spend all 10
    assert!(!game.buy_ad1()); // can't afford the next one (cost 100)
}

#[test]
fn test_ad1_produces_antimatter() {
    let mut game = GameState::new();
    game.buy_ad1(); // Now have 1 AD1, 0 antimatter

    // Tick 1 second: should gain 1 antimatter (1 AD1 * 1s)
    game.tick(1000.0);
    assert_eq!(game.antimatter, Decimal::from_float(1.0));
}

#[test]
fn test_ad1_production_scales_with_time() {
    let mut game = GameState::new();
    game.buy_ad1();

    // Tick 5 seconds: should gain 5 antimatter
    game.tick(5000.0);
    assert_eq!(game.antimatter, Decimal::from_float(5.0));
}
