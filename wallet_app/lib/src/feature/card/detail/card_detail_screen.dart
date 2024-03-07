import 'package:fimber/fimber.dart';
import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import '../../../domain/model/attribute/attribute.dart';
import '../../../domain/model/timeline/interaction_timeline_attribute.dart';
import '../../../domain/model/timeline/operation_timeline_attribute.dart';
import '../../../domain/model/wallet_card.dart';
import '../../../domain/model/wallet_card_detail.dart';
import '../../../navigation/wallet_routes.dart';
import '../../../util/extension/animation_extension.dart';
import '../../../util/extension/build_context_extension.dart';
import '../../../util/formatter/card_valid_until_time_formatter.dart';
import '../../../util/formatter/operation_issued_time_formatter.dart';
import '../../../util/formatter/time_ago_formatter.dart';
import '../../../util/formatter/timeline_attribute_status_formatter.dart';
import '../../common/screen/placeholder_screen.dart';
import '../../common/sheet/explanation_sheet.dart';
import '../../common/widget/animated_fade_in.dart';
import '../../common/widget/button/bottom_back_button.dart';
import '../../common/widget/card/wallet_card_item.dart';
import '../../common/widget/centered_loading_indicator.dart';
import '../../common/widget/info_row.dart';
import '../../common/widget/sliver_divider.dart';
import '../../common/widget/sliver_sized_box.dart';
import '../../common/widget/wallet_app_bar.dart';
import '../data/argument/card_data_screen_argument.dart';
import 'argument/card_detail_screen_argument.dart';
import 'bloc/card_detail_bloc.dart';

/// This value can be used with [SecuredPageRoute.overrideDurationOfNextTransition] when navigating to the
/// [CardDetailScreen] to slow down the entry transition a bit, making it feel a bit less rushed when the card
/// animates into place.
const kPreferredCardDetailEntryTransitionDuration = Duration(milliseconds: 600);
const _kCardExpiresInDays = 365; // 1 year for demo purposes

class CardDetailScreen extends StatelessWidget {
  static CardDetailScreenArgument getArgument(RouteSettings settings) {
    final args = settings.arguments;
    try {
      return CardDetailScreenArgument.fromJson(args as Map<String, dynamic>);
    } catch (exception, stacktrace) {
      Fimber.e('Failed to decode type: ${args.runtimeType} arg: $args', ex: exception, stacktrace: stacktrace);
      throw UnsupportedError(
          'Make sure to pass in [CardDetailScreenArgument] as json when opening the CardDetailScreen');
    }
  }

  final String cardTitle;

  const CardDetailScreen({required this.cardTitle, super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      key: const Key('cardDetailScreen'),
      appBar: _buildAppBar(context),
      body: SafeArea(
        child: _buildBody(context),
      ),
    );
  }

  PreferredSizeWidget _buildAppBar(BuildContext context) {
    final fallbackAppBarTitleText = Text(cardTitle);
    return WalletAppBar(
      title: BlocBuilder<CardDetailBloc, CardDetailState>(
        builder: (context, state) {
          return switch (state) {
            CardDetailInitial() => fallbackAppBarTitleText,
            CardDetailLoadInProgress() => fallbackAppBarTitleText,
            CardDetailLoadSuccess() => Text(state.detail.card.front.title.l10nValue(context)),
            CardDetailLoadFailure() => fallbackAppBarTitleText,
          };
        },
      ),
    );
  }

  Widget _buildBody(BuildContext context) {
    return BlocBuilder<CardDetailBloc, CardDetailState>(
      builder: (context, state) {
        return switch (state) {
          CardDetailInitial() => _buildLoading(context),
          CardDetailLoadInProgress() => _buildLoading(context, card: state.card),
          CardDetailLoadSuccess() => _buildDetail(context, state.detail),
          CardDetailLoadFailure() => _buildError(context, state),
        };
      },
    );
  }

  Widget _buildLoading(BuildContext context, {WalletCard? card}) {
    if (card == null) return const CenteredLoadingIndicator();
    return CustomScrollView(
      slivers: [
        const SliverSizedBox(height: 24 + 8),
        SliverToBoxAdapter(
          child: ExcludeSemantics(
            child: FractionallySizedBox(
              widthFactor: 0.6,
              child: Hero(
                tag: card.id,
                flightShuttleBuilder: (
                  BuildContext flightContext,
                  Animation<double> animation,
                  HeroFlightDirection flightDirection,
                  BuildContext fromHeroContext,
                  BuildContext toHeroContext,
                ) {
                  animation
                      .addOnCompleteListener(() => context.read<CardDetailBloc>().notifyEntryTransitionCompleted());
                  return WalletCardItem.buildShuttleCard(animation, card.front, ctaAnimation: CtaAnimation.fadeOut);
                },
                child: WalletCardItem.fromCardFront(context: context, front: card.front),
              ),
            ),
          ),
        ),
        const SliverSizedBox(height: 32),
        const SliverDivider(height: 1),
        const SliverFillRemaining(
          child: CenteredLoadingIndicator(),
        ),
      ],
    );
  }

  Widget _buildDetail(BuildContext context, WalletCardDetail detail) {
    final card = detail.card;

    return Column(
      children: [
        Expanded(
          child: Scrollbar(
            child: ListView(
              padding: const EdgeInsets.only(top: 24),
              children: [
                const SizedBox(height: 8),
                ExcludeSemantics(
                  child: FractionallySizedBox(
                    widthFactor: 0.6,
                    child: Hero(
                      tag: card.id,
                      flightShuttleBuilder: (
                        BuildContext flightContext,
                        Animation<double> animation,
                        HeroFlightDirection flightDirection,
                        BuildContext fromHeroContext,
                        BuildContext toHeroContext,
                      ) =>
                          WalletCardItem.buildShuttleCard(animation, card.front, ctaAnimation: CtaAnimation.fadeIn),
                      child: WalletCardItem.fromCardFront(context: context, front: card.front),
                    ),
                  ),
                ),
                const SizedBox(height: 32),
                const Divider(height: 1),
                AnimatedFadeIn(
                  child: _buildDetailContent(context, detail),
                ),
              ],
            ),
          ),
        ),
        const BottomBackButton(),
      ],
    );
  }

  Widget _buildDetailContent(BuildContext context, WalletCardDetail detail) {
    final card = detail.card;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        InfoRow(
          icon: Icons.description_outlined,
          title: Text(context.l10n.cardDetailScreenCardDataCta),
          subtitle: Text(context.l10n.cardDetailScreenCardDataIssuedBy(card.issuer.displayName.l10nValue(context))),
          onTap: () => _onCardDataPressed(context, card),
        ),
        const Divider(height: 1),
        InfoRow(
          icon: Icons.history_outlined,
          title: Text(context.l10n.cardDetailScreenCardHistoryCta),
          subtitle: Text(_createInteractionText(context, detail.latestSuccessInteraction)),
          onTap: () => _onCardHistoryPressed(context, card.docType),
        ),
        const Divider(height: 1),
        if (card.config.updatable) ...[
          InfoRow(
            icon: Icons.replay_outlined,
            title: Text(context.l10n.cardDetailScreenCardUpdateCta),
            subtitle: Text(_createOperationText(context, detail.latestIssuedOperation)),
            onTap: () => _onCardUpdatePressed(context, card),
          ),
          const Divider(height: 1),
        ],
        if (card.config.removable) ...[
          InfoRow(
            icon: Icons.delete_outline_rounded,
            title: Text(context.l10n.cardDetailScreenCardDeleteCta),
            onTap: () => _onCardDeletePressed(context),
          ),
          const Divider(height: 1)
        ],
      ],
    );
  }

  void _showNoUpdateAvailableSheet(BuildContext context) {
    ExplanationSheet.show(
      context,
      title: context.l10n.cardDetailScreenNoUpdateAvailableSheetTitle,
      description: context.l10n.cardDetailScreenNoUpdateAvailableSheetDescription,
      closeButtonText: context.l10n.cardDetailScreenNoUpdateAvailableSheetCloseCta,
    );
  }

  String _createInteractionText(BuildContext context, InteractionTimelineAttribute? attribute) {
    if (attribute != null) {
      final String timeAgo = TimeAgoFormatter.format(context, attribute.dateTime);
      final String status = TimelineAttributeStatusTextFormatter.map(context, attribute).toLowerCase();
      return context.l10n.cardDetailScreenLatestSuccessInteraction(
        attribute.organization.displayName.l10nValue(context),
        status,
        timeAgo,
      );
    } else {
      return context.l10n.cardDetailScreenLatestSuccessInteractionUnknown;
    }
  }

  String _createOperationText(BuildContext context, OperationTimelineAttribute? attribute) {
    if (attribute != null) {
      DateTime issued = attribute.dateTime;
      String issuedTime = OperationIssuedTimeFormatter.format(context, issued);
      String issuedText = context.l10n.cardDetailScreenLatestIssuedOperation(issuedTime);

      DateTime validUntil = issued.add(const Duration(days: _kCardExpiresInDays));
      String validUntilTime = CardValidUntilTimeFormatter.format(context, validUntil);
      String validUntilText = context.l10n.cardDetailScreenCardValidUntil(validUntilTime);

      return '$issuedText\n$validUntilText';
    } else {
      return context.l10n.cardDetailScreenLatestIssuedOperationUnknown;
    }
  }

  Widget _buildError(BuildContext context, CardDetailLoadFailure state) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline),
          const SizedBox(height: 16),
          TextButton(
            child: Text(context.l10n.generalRetry),
            onPressed: () => context.read<CardDetailBloc>().add(CardDetailLoadTriggered(state.cardId)),
          ),
        ],
      ),
    );
  }

  void _onCardDataPressed(BuildContext context, WalletCard card) {
    Navigator.restorablePushNamed(
      context,
      WalletRoutes.cardDataRoute,
      arguments: CardDataScreenArgument(
        cardId: card.id,
        cardTitle: card.front.title.l10nValue(context),
      ).toMap(),
    );
  }

  void _onCardHistoryPressed(BuildContext context, String docType) {
    Navigator.pushNamed(
      context,
      WalletRoutes.cardHistoryRoute,
      arguments: docType,
    );
  }

  void _onCardUpdatePressed(BuildContext context, WalletCard card) {
    _showNoUpdateAvailableSheet(context);
  }

  void _onCardDeletePressed(BuildContext context) {
    PlaceholderScreen.show(context);
  }
}
