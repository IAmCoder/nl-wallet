package feature.personalize

import helper.TestBase
import navigator.OnboardingNavigator
import navigator.screen.OnboardingScreen
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.DisplayName
import org.junitpioneer.jupiter.RetryingTest
import screen.personalize.PersonalizeAuthenticatingWithDigidScreen
import screen.personalize.PersonalizeInformScreen

@DisplayName("${PersonalizeAuthenticatingWithDigidScreenTests.USE_CASE} App performs issuance with PID provider [${PersonalizeAuthenticatingWithDigidScreenTests.JIRA_ID}]")
class PersonalizeAuthenticatingWithDigidScreenTests : TestBase() {

    companion object {
        const val USE_CASE = "UC 3.1"
        const val JIRA_ID = "PVW-1036"
    }

    private lateinit var personalizeAuthenticatingWithDigidScreen: PersonalizeAuthenticatingWithDigidScreen

    @BeforeEach
    fun setUp() {
        OnboardingNavigator().toScreen(OnboardingScreen.PersonalizeInform)

        PersonalizeInformScreen().clickLoginWithDigidButton(false)

        personalizeAuthenticatingWithDigidScreen = PersonalizeAuthenticatingWithDigidScreen()
    }

    @RetryingTest(value = MAX_RETRY_COUNT, name = "{displayName} - {index}")
    @DisplayName("$USE_CASE.1 The App displays a loading screen whilst this process is pending. [$JIRA_ID]")
    fun verifyPersonalizeAuthenticatingWithDigidScreen() {
        assertTrue(
            personalizeAuthenticatingWithDigidScreen.visible(),
            "personalize authenticating with digid screen is not visible"
        )
    }

    /**
     * 2. The App requests PID from the PID Provider by providing the OIDC access token that resulted from the DigiD login.
     * >> This requirement hard, if not impossible to be tested in an e2e setup and should be validated during an audit of the app.
     */

    /**
     * 3. The issuance protocol and format are in accordance with the specifications described in PVW-1059.
     * >> This requirement hard, if not impossible to be tested in an e2e setup and should be validated during an audit of the app.
     */

    /**
     * 4. Go to FEAT 'User confirms/rejects offered PID'.
     * >> Covered in PersonalizePidPreviewScreenTests.
     */

    /**
     * 5. Upon user confirmation, the PID Provider issues the PID to the App.
     * >> Duplicate requirement from PersonalizePidPreviewScreenTests
     */

    /**
     * 6. When PID/address issuance fails, the App displays an appropriate message to the User.
     * >> Manual test: https://SSSS/jira/browse/PVW-1769
     */
}
