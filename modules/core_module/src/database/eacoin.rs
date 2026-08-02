use bson::{DateTime, doc};
use serde::Serialize;

use super::{CARDS, Database, LEDGER, card::CardDocument, decode, encode};

impl Database {
    pub(crate) async fn eacoin_balance(&self, card_id: String) -> Result<Option<i32>, String> {
        self.raw
            .find_one(
                CARDS.into(),
                doc! { "_id": card_id, "active": true },
                Default::default(),
            )
            .await?
            .map(decode::<CardDocument>)
            .transpose()
            .map(|card| card.map(|card| card.balance))
    }

    pub(crate) async fn consume_eacoin(
        &self,
        input: ConsumeInput,
    ) -> Result<Option<ConsumeResult>, String> {
        let payment = input.payment;
        let before = self
            .raw
            .find_one_and_update(
                CARDS.into(),
                doc! { "_id": &input.card_id, "active": true, "balance": { "$gte": payment } },
                vec![doc! {
                    "$set": {
                        "balance": {
                            "$let": {
                                "vars": { "next": { "$subtract": ["$balance", payment] } },
                                "in": {
                                    "$cond": [
                                        { "$and": ["$auto_charge_enabled", { "$lt": ["$$next", "$auto_charge_threshold"] }] },
                                        { "$add": ["$$next", "$auto_charge_amount"] },
                                        "$$next"
                                    ]
                                }
                            }
                        }
                    }
                }]
                .into(),
                Default::default(),
            )
            .await?
            .map(decode::<CardDocument>)
            .transpose()?;
        if let Some(card) = before {
            self.raw
                .insert_one(
                    LEDGER.into(),
                    encode(&LedgerDocument {
                        card_id: input.card_id,
                        pcb_id: input.pcb_id,
                        payment,
                        service: input.service,
                        item_type: input.item_type,
                        detail: input.detail,
                        created_at: DateTime::now(),
                    })?,
                )
                .await?;
            return Ok(Some(ConsumeResult {
                balance: card.balance - payment,
                accepted: true,
                autocharge: card.auto_charge_enabled,
            }));
        }
        self.raw
            .find_one(
                CARDS.into(),
                doc! { "_id": &input.card_id, "active": true },
                Default::default(),
            )
            .await?
            .map(decode::<CardDocument>)
            .transpose()
            .map(|card| {
                card.map(|card| ConsumeResult {
                    balance: card.balance - payment,
                    accepted: false,
                    autocharge: card.auto_charge_enabled,
                })
            })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConsumeInput {
    pub(crate) card_id: String,
    pub(crate) pcb_id: String,
    pub(crate) payment: i32,
    pub(crate) service: i32,
    pub(crate) item_type: String,
    pub(crate) detail: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ConsumeResult {
    pub(crate) balance: i32,
    pub(crate) accepted: bool,
    pub(crate) autocharge: bool,
}

#[derive(Clone, Debug, Serialize)]
struct LedgerDocument {
    card_id: String,
    pcb_id: String,
    payment: i32,
    service: i32,
    item_type: String,
    detail: String,
    created_at: DateTime,
}
