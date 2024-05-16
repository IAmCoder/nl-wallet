import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golden_toolkit/golden_toolkit.dart';
import 'package:wallet/src/domain/model/attribute/attribute.dart';
import 'package:wallet/src/domain/model/event/wallet_event.dart';
import 'package:wallet/src/feature/common/widget/history/wallet_event_row.dart';
import 'package:wallet/src/util/extension/string_extension.dart';

import '../../../../../wallet_app_test_widget.dart';
import '../../../../mocks/wallet_mock_data.dart';

void main() {
  const kGoldenSize = Size(350, 115);

  group('goldens', () {
    testGoldens(
      'light wallet_event operation issued',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletMockData.issuanceEvent,
            onPressed: () {},
          ),
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.operation.issued');
      },
    );
    testGoldens(
      'dark wallet_event operation issued',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletMockData.issuanceEvent,
            onPressed: () {},
          ),
          brightness: Brightness.dark,
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/dark.operation.issued');
      },
    );

    testGoldens(
      'light wallet_event interaction success',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletMockData.disclosureEvent,
            onPressed: () {},
          ),
          surfaceSize: const Size(350, 89),
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.interaction.success');
      },
    );

    testGoldens(
      'light wallet_event interaction failed',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletEvent.disclosure(
              dateTime: DateTime(2024),
              status: EventStatus.error,
              relyingParty: WalletMockData.organization,
              purpose: 'disclosure'.untranslated,
              cards: [WalletMockData.card],
              policy: WalletMockData.policy,
              disclosureType: DisclosureType.regular,
            ),
            onPressed: () {},
          ),
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.interaction.failed');
      },
    );

    testGoldens(
      'light wallet_event interaction rejected',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletEvent.disclosure(
              dateTime: DateTime(2024),
              status: EventStatus.cancelled,
              relyingParty: WalletMockData.organization,
              purpose: 'disclosure'.untranslated,
              cards: [WalletMockData.card],
              policy: WalletMockData.policy,
              disclosureType: DisclosureType.regular,
            ),
            onPressed: () {},
          ),
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.interaction.rejected');
      },
    );

    testGoldens(
      'light wallet_event signing success',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletMockData.signEvent,
            onPressed: () {},
          ),
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.signing.success');
      },
    );
    testGoldens(
      'light wallet_event signing rejected',
      (tester) async {
        await tester.pumpWidgetWithAppWrapper(
          WalletEventRow(
            event: WalletEvent.sign(
              dateTime: DateTime(2024),
              status: EventStatus.cancelled,
              relyingParty: WalletMockData.organization,
              policy: WalletMockData.policy,
              document: WalletMockData.document,
            ),
            onPressed: () {},
          ),
          surfaceSize: kGoldenSize,
        );
        await screenMatchesGolden(tester, 'wallet_event_row/light.signing.rejected');
      },
    );
  });

  group('widgets', () {
    testWidgets('onPressed is triggered', (tester) async {
      bool tapped = false;
      await tester.pumpWidgetWithAppWrapper(
        WalletEventRow(
          event: WalletMockData.issuanceEvent,
          onPressed: () => tapped = true,
        ),
      );

      // Validate that the widget exists
      final titleFinder = find.text(WalletMockData.issuanceEvent.card.front.title.testValue);
      // Finds both the row title and the title in the card thumbnail
      expect(titleFinder, findsNWidgets(2));

      // Tap any title, as the whole row should be clickable
      await tester.tap(titleFinder.last);
      expect(tapped, true, reason: 'onPressed was not called');
    });
  });
}
