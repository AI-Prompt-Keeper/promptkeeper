package com.promptkeeper.sdk

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class PromptKeeperTest {

    @Test
    fun initialize_setsDefaultInstance() {
        val sdk = PromptKeeper.initialize(apiKey = "pk_test")
        assertNotNull(PromptKeeper.getInstance())
        assertEquals(sdk, PromptKeeper.getInstance())
    }

    @Test
    fun constructor_createsInstance() {
        val sdk = PromptKeeper(apiKey = "pk_other")
        assertNotNull(sdk)
    }
}
