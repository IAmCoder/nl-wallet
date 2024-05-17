import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../domain/model/event/wallet_event.dart';
import '../../../domain/model/wallet_card.dart';
import '../../../navigation/wallet_routes.dart';
import '../../../util/extension/build_context_extension.dart';
import '../../../util/extension/wallet_event_extension.dart';
import '../../common/widget/button/bottom_back_button.dart';
import '../../common/widget/centered_loading_indicator.dart';
import '../../common/widget/history/history_section_sliver.dart';
import '../../common/widget/sliver_sized_box.dart';
import '../../common/widget/sliver_wallet_app_bar.dart';
import '../../history/detail/argument/history_detail_screen_argument.dart';
import 'bloc/card_history_bloc.dart';

class CardHistoryScreen extends StatelessWidget {
  static String getArguments(RouteSettings settings) {
    final args = settings.arguments;
    try {
      return args as String;
    } catch (exception, stacktrace) {
      Fimber.e('Failed to decode $args', ex: exception, stacktrace: stacktrace);
      throw UnsupportedError('Make sure to pass in a (mock) id when opening the CardHistoryScreen');
    }
  }

  const CardHistoryScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      key: const Key('cardHistoryScreen'),
      body: SafeArea(
        child: Column(
          children: [
            Expanded(child: _buildContent(context)),
            const BottomBackButton(),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(BuildContext context) {
    return BlocBuilder<CardHistoryBloc, CardHistoryState>(
      builder: (context, state) {
        final sliver = switch (state) {
          CardHistoryInitial() => _buildLoadingSliver(),
          CardHistoryLoadInProgress() => _buildLoadingSliver(),
          CardHistoryLoadSuccess() => _buildSuccessSliver(context, state),
          CardHistoryLoadFailure() => _buildErrorSliver(context),
        };
        return Scrollbar(
          child: CustomScrollView(
            slivers: [
              SliverWalletAppBar(title: context.l10n.cardHistoryScreenTitle),
              sliver,
            ],
          ),
        );
      },
    );
  }

  Widget _buildLoadingSliver() {
    return const SliverFillRemaining(
      child: CenteredLoadingIndicator(),
    );
  }

  Widget _buildSuccessSliver(BuildContext context, CardHistoryLoadSuccess state) {
    List<Widget> sections = state.events.sectionedByMonth
        .map(
          (section) => HistorySectionSliver(
            section: section,
            onRowPressed: (event) => _onEventPressed(context, event, state.card),
          ),
        )
        .toList();
    return SliverMainAxisGroup(slivers: [...sections, const SliverSizedBox(height: 24)]);
  }

  void _onEventPressed(BuildContext context, WalletEvent event, WalletCard card) {
    Navigator.pushNamed(
      context,
      WalletRoutes.historyDetailRoute,
      arguments: HistoryDetailScreenArgument(
        walletEvent: event,
        docType: card.docType,
      ).toMap(),
    );
  }

  Widget _buildErrorSliver(BuildContext context) {
    return SliverFillRemaining(
      hasScrollBody: false,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            const Spacer(),
            Text(
              context.l10n.errorScreenGenericDescription,
              textAlign: TextAlign.center,
            ),
            const Spacer(),
            ElevatedButton(
              onPressed: () {
                final settings = ModalRoute.of(context)?.settings;
                if (settings != null) {
                  final cardId = getArguments(settings);
                  context.read<CardHistoryBloc>().add(CardHistoryLoadTriggered(cardId));
                } else {
                  Navigator.pop(context);
                }
              },
              child: Text(context.l10n.generalRetry),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}
