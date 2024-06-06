import 'package:flutter_test/flutter_test.dart';
import 'package:wallet/src/feature/common/dialog/scan_with_wallet_dialog.dart';

import '../../../../wallet_app_test_widget.dart';
import '../../../util/test_utils.dart';

void main() {
  testWidgets('ScanWithWalletDialog shows expected copy', (tester) async {
    await tester.pumpWidget(
      const WalletAppTestWidget(
        child: ScanWithWalletDialog(),
      ),
    );

    final l10n = await TestUtils.englishLocalizations;

    // Setup finders
    final titleFinder = find.text(l10n.scanWithWalletDialogTitle);
    final descriptionFinder = find.text(l10n.scanWithWalletDialogBody);
    final ctaFinder = find.text(l10n.scanWithWalletDialogScanCta.toUpperCase());

    // Verify all expected widgets show up once
    expect(titleFinder, findsOneWidget);
    expect(descriptionFinder, findsOneWidget);
    expect(ctaFinder, findsOneWidget);
  });
}
