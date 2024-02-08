import 'package:bloc_test/bloc_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golden_toolkit/golden_toolkit.dart';
import 'package:wallet/src/domain/model/attribute/attribute.dart';
import 'package:wallet/src/feature/history/overview/bloc/history_overview_bloc.dart';
import 'package:wallet/src/feature/history/overview/history_overview_screen.dart';

import '../../../../wallet_app_test_widget.dart';
import '../../../mocks/wallet_mock_data.dart';
import '../../../util/device_utils.dart';

class MockHistoryOverviewBloc extends MockBloc<HistoryOverviewEvent, HistoryOverviewState>
    implements HistoryOverviewBloc {}

void main() {
  final historyOverviewLoadSuccessMock = HistoryOverviewLoadSuccess([
    WalletMockData.operationTimelineAttribute,
    WalletMockData.signingTimelineAttribute,
    WalletMockData.interactionTimelineAttribute,
  ]);

  group('goldens', () {
    testGoldens('HistoryOverviewLoadSuccess light', (tester) async {
      await tester.pumpDeviceBuilder(
        DeviceUtils.deviceBuilderWithPrimaryScrollController
          ..addScenario(
            widget: const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
              MockHistoryOverviewBloc(),
              historyOverviewLoadSuccessMock,
            ),
          ),
        wrapper: walletAppWrapper(),
      );
      await screenMatchesGolden(tester, 'success.light');
    });

    testGoldens('HistoryOverviewLoadSuccess dark', (tester) async {
      await tester.pumpDeviceBuilder(
        DeviceUtils.deviceBuilderWithPrimaryScrollController
          ..addScenario(
            widget: const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
              MockHistoryOverviewBloc(),
              historyOverviewLoadSuccessMock,
            ),
          ),
        wrapper: walletAppWrapper(brightness: Brightness.dark),
      );
      await screenMatchesGolden(tester, 'success.dark');
    });

    testGoldens('HistoryOverviewLoadInProgress light', (tester) async {
      await tester.pumpWidgetWithAppWrapper(
        const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
          MockHistoryOverviewBloc(),
          const HistoryOverviewLoadInProgress(),
        ),
      );
      await screenMatchesGolden(tester, 'loading.light');
    });

    testGoldens('HistoryOverviewLoadFailure light', (tester) async {
      await tester.pumpWidgetWithAppWrapper(
        const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
          MockHistoryOverviewBloc(),
          const HistoryOverviewLoadFailure(),
        ),
      );
      await screenMatchesGolden(tester, 'error.light');
    });
  });

  group('widgets', () {
    testWidgets('OperationAttribute renders the card title', (tester) async {
      await tester.pumpWidgetWithAppWrapper(
        const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
          MockHistoryOverviewBloc(),
          HistoryOverviewLoadSuccess([
            WalletMockData.operationTimelineAttribute,
          ]),
        ),
      );

      // Operation renders the title of the card twice, once as the row title, and once inside the card thumbnail
      expect(find.text(WalletMockData.operationTimelineAttribute.card.front.title.testValue), findsNWidgets(2));
    });

    testWidgets('SignAttribute renders the organization title', (tester) async {
      await tester.pumpWidgetWithAppWrapper(
        const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
          MockHistoryOverviewBloc(),
          HistoryOverviewLoadSuccess([
            WalletMockData.signingTimelineAttribute,
          ]),
        ),
      );

      // Sign attribute renders the title of the organization
      expect(find.text(WalletMockData.organization.displayName.testValue), findsOneWidget);
    });

    testWidgets('InteractionAttribute renders the organization title', (tester) async {
      await tester.pumpWidgetWithAppWrapper(
        const HistoryOverviewScreen().withState<HistoryOverviewBloc, HistoryOverviewState>(
          MockHistoryOverviewBloc(),
          HistoryOverviewLoadSuccess([
            WalletMockData.interactionTimelineAttribute,
          ]),
        ),
      );

      // Interaction attribute renders the title of the organization
      expect(find.text(WalletMockData.organization.displayName.testValue), findsOneWidget);
    });
  });
}
