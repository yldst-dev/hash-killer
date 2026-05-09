import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';

import '../../../core/design_system/app_button.dart';
import '../../../core/design_system/app_dialog.dart';
import '../../../core/design_system/app_empty_state.dart';
import '../../../core/design_system/app_icon_box.dart';
import '../../../core/design_system/app_icons.dart';
import '../../../core/design_system/app_item.dart';
import '../../../core/design_system/app_panel.dart';
import '../../../core/design_system/app_progress.dart';
import '../../../core/design_system/app_scaffold.dart';
import '../../../core/design_system/app_selection.dart';
import '../../../core/design_system/app_text_field.dart';
import '../../../core/design_system/app_tokens.dart';
import '../application/scan_controller.dart';
import '../application/scan_state.dart';
import '../domain/duplicate_relation.dart';
import '../domain/hash_algorithm.dart';
import '../domain/scan_formatters.dart';
import '../domain/scan_mode.dart';
import '../domain/scan_report.dart';
import '../domain/volume_destination.dart';

class ScanPage extends ConsumerWidget {
  const ScanPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(scanControllerProvider);
    final controller = ref.read(scanControllerProvider.notifier);

    return AppScaffold(
      child: ColoredBox(
        color: context.theme.colors.background,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final width = MediaQuery.sizeOf(context).width;
            final contentHeight = constraints.maxHeight - 16;
            final dashboardHeight = contentHeight < 680 ? 680.0 : contentHeight;

            return Stack(
              children: [
                SingleChildScrollView(
                  padding: const EdgeInsets.all(8),
                  child: ConstrainedBox(
                    constraints: BoxConstraints(minHeight: dashboardHeight),
                    child: _DashboardLayout(
                      state: state,
                      controller: controller,
                      compact: width < context.theme.breakpoints.lg,
                      availableHeight: dashboardHeight,
                    ),
                  ),
                ),
                _DialogHost(state: state, controller: controller),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _DashboardLayout extends StatelessWidget {
  const _DashboardLayout({
    required this.state,
    required this.controller,
    required this.compact,
    required this.availableHeight,
  });

  final ScanState state;
  final ScanController controller;
  final bool compact;
  final double availableHeight;

  @override
  Widget build(BuildContext context) {
    final extraHeight = compact ? 0.0 : availableHeight - 680;
    final mainHeight = 386 + extraHeight * 0.72;
    final activityHeight = 154 + extraHeight * 0.28;

    return Column(
      children: [
        _PathCard(state: state, controller: controller),
        const SizedBox(height: 12),
        if (compact)
          Column(
            children: [
              _SettingsCard(state: state, controller: controller),
              const SizedBox(height: 12),
              SizedBox(height: 210, child: _ProgressCard(state: state)),
              const SizedBox(height: 12),
              SizedBox(
                height: 430,
                child: _ResultCard(state: state, controller: controller),
              ),
            ],
          )
        else
          SizedBox(
            height: mainHeight,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  flex: 13,
                  child: Column(
                    children: [
                      _SettingsCard(state: state, controller: controller),
                      const SizedBox(height: 12),
                      Expanded(child: _ProgressCard(state: state)),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  flex: 8,
                  child: _ResultCard(state: state, controller: controller),
                ),
              ],
            ),
          ),
        const SizedBox(height: 12),
        SizedBox(
          height: compact ? 154 : activityHeight,
          child: _ActivityCard(state: state, controller: controller),
        ),
        const SizedBox(height: 12),
        _FooterRow(state: state, controller: controller),
      ],
    );
  }
}

class _PathCard extends StatelessWidget {
  const _PathCard({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    final title = switch (state.paths.length) {
      0 => '선택된 경로 없음',
      1 => state.paths.first,
      final count => '$count개 경로 선택',
    };

    return SizedBox(
      height: 64,
      child: _Panel(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        backgroundColor: AppTokens.muted,
        child: Row(
          children: [
            const AppIconBox(
              icon: AppIcons.folder,
              size: AppIconBoxSize.regular,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: AppTokens.foreground,
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            const SizedBox(width: 12),
            SizedBox(
              width: 190,
              child: AppButton(
                label: '폴더 선택',
                tone: AppButtonTone.accentOutline,
                icon: AppIcons.folderOpen,
                expanded: true,
                onPressed: state.running ? null : controller.pickFolders,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  const _SettingsCard({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          const AppSectionHeader(icon: AppIcons.settings, label: '검사 설정'),
          _SettingRow(
            icon: AppIcons.speed,
            label: '검사 모드',
            value: state.scanMode.label,
            disabled: state.running,
            onPressed: () => controller.openDialog(ScanDialog.scanModeSettings),
          ),
          _SettingRow(
            icon: AppIcons.tag,
            label: '비교 기준',
            value: state.algorithm.label,
            disabled: state.running,
            onPressed: () =>
                controller.openDialog(ScanDialog.algorithmSettings),
          ),
          _SettingRow(
            icon: AppIcons.folder,
            label: '검사 경로',
            value: _pathBadge(state.paths),
            disabled: state.running,
            onPressed: () => controller.openDialog(ScanDialog.pathList),
          ),
          _SettingRow(
            icon: AppIcons.cache,
            label: '캐시 제한',
            value: '${state.cacheLimitMb} MB',
            disabled: state.running,
            onPressed: () => controller.openDialog(ScanDialog.cacheSettings),
          ),
          _SettingRow(
            icon: AppIcons.archive,
            label: '보관 폴더',
            value: _quarantineTitle(state.quarantineDestinations),
            disabled: state.running,
            showBottomBorder: false,
            onPressed: () =>
                controller.openDialog(ScanDialog.quarantineSettings),
          ),
        ],
      ),
    );
  }

  String _pathBadge(List<String> paths) {
    return switch (paths.length) {
      0 => '선택된 경로 없음',
      final count => '$count개 경로 선택됨',
    };
  }

  String _quarantineTitle(List<VolumeDestination> destinations) {
    if (destinations.isEmpty) {
      return '미지정';
    }

    final configured = destinations
        .where((destination) => destination.configured)
        .length;

    if (configured == destinations.length) {
      return '$configured개 경로 지정됨';
    }

    return '$configured/${destinations.length}개 경로 지정됨';
  }
}

class _ProgressCard extends StatelessWidget {
  const _ProgressCard({required this.state});

  final ScanState state;

  @override
  Widget build(BuildContext context) {
    final progressText = '${(state.progress * 100).toStringAsFixed(1)}%';
    final statusLabel = state.running
        ? '진행 중'
        : state.report != null
        ? '완료'
        : '대기 중';

    return _Panel(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          const AppSectionHeader(icon: AppIcons.activity, label: '진행 상태'),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
              child: _Panel(
                padding: EdgeInsets.zero,
                child: Column(
                  children: [
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 10, 12, 12),
                      child: Column(
                        children: [
                          Row(
                            children: [
                              if (state.running) ...[
                                const AppSpinner(),
                                const SizedBox(width: 8),
                              ],
                              Text(
                                progressText,
                                style: const TextStyle(
                                  color: AppTokens.primary,
                                  fontSize: 20,
                                  fontWeight: FontWeight.w700,
                                ),
                              ),
                              const Spacer(),
                              Text(
                                '${state.processed}/${state.total} 파일 처리',
                                style: const TextStyle(
                                  color: AppTokens.foreground,
                                  fontSize: 13,
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 8),
                          AppProgress(
                            value: state.running && state.scanProgress == null
                                ? null
                                : state.progress,
                          ),
                        ],
                      ),
                    ),
                    const DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          top: BorderSide(color: AppTokens.border),
                        ),
                      ),
                      child: SizedBox(height: 1),
                    ),
                    Expanded(
                      child: Row(
                        children: [
                          Expanded(
                            child: _MiniStat(
                              icon: AppIcons.file,
                              label: '처리된 파일',
                              value: '${state.processed} / ${state.total}',
                            ),
                          ),
                          const _VerticalDivider(),
                          const Expanded(
                            child: _MiniStat(
                              icon: AppIcons.timer,
                              label: '총 검사 시간',
                              value: '대기',
                            ),
                          ),
                          const _VerticalDivider(),
                          Expanded(
                            child: _MiniStat(
                              icon: AppIcons.check,
                              label: '상태',
                              value: statusLabel,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _VerticalDivider extends StatelessWidget {
  const _VerticalDivider();

  @override
  Widget build(BuildContext context) {
    return const SizedBox(
      width: 1,
      height: 32,
      child: DecoratedBox(decoration: BoxDecoration(color: AppTokens.border)),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    final report = state.report ?? const ScanReport.empty();

    return _Panel(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          const AppSectionHeader(icon: AppIcons.barChart, label: '결과 요약'),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 40, 12, 12),
              child: Column(
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.search,
                          tone: AppIconTone.blue,
                          label: '스캔',
                          value: '${report.scannedFiles}',
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.list,
                          tone: AppIconTone.purple,
                          label: '후보',
                          value: '${report.candidateFiles}',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Row(
                    children: [
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.hash,
                          tone: AppIconTone.green,
                          label: '해시',
                          value: '${report.hashedFiles}',
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.cache,
                          tone: AppIconTone.orange,
                          label: '캐시',
                          value: '${report.reusedHashes}',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Row(
                    children: [
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.users,
                          tone: AppIconTone.yellow,
                          label: '그룹',
                          value: '${report.duplicateGroups}',
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: _ResultRow(
                          icon: AppIcons.archive,
                          tone: AppIconTone.emerald,
                          label: '회수',
                          value: formatBytes(report.reclaimedBytes),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  _ResultRow(
                    icon: AppIcons.trash,
                    tone: AppIconTone.red,
                    label: '삭제',
                    value: '${report.deletedFiles}',
                  ),
                ],
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
            child: SizedBox(
              width: double.infinity,
              child: AppButton(
                label: '중복 관계 보기',
                expanded: true,
                icon: AppIcons.right,
                iconPosition: AppButtonIconPosition.trailing,
                tone: AppButtonTone.outline,
                onPressed: state.report == null
                    ? null
                    : () =>
                          controller.openDialog(ScanDialog.duplicateRelations),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ActivityCard extends StatefulWidget {
  const _ActivityCard({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  State<_ActivityCard> createState() => _ActivityCardState();
}

class _ActivityCardState extends State<_ActivityCard> {
  final _scrollController = ScrollController();
  int _lastEventCount = 0;

  @override
  void didUpdateWidget(covariant _ActivityCard oldWidget) {
    super.didUpdateWidget(oldWidget);

    final eventCount = widget.state.activityEvents.length;
    if (eventCount > _lastEventCount) {
      _lastEventCount = eventCount;
      WidgetsBinding.instance.addPostFrameCallback((_) => _scrollToLatest());
    } else {
      _lastEventCount = eventCount;
    }
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final events = widget.state.activityEvents;
    final logEvents = widget.state.activityLogEvents;

    return _Panel(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          AppSectionHeader(
            icon: AppIcons.logs,
            label: '실시간 작업',
            action: AppButton(
              label: '로그 저장',
              tone: AppButtonTone.ghost,
              size: AppButtonSize.small,
              onPressed: logEvents.isEmpty
                  ? null
                  : widget.controller.exportActivityLog,
            ),
          ),
          Expanded(
            child: events.isEmpty
                ? const _ActivityEmpty()
                : ListView.separated(
                    controller: _scrollController,
                    padding: const EdgeInsets.all(12),
                    itemBuilder: (context, index) {
                      final event = events[index];

                      return _ActivityRow(
                        stage: event.stage,
                        detail: event.detail,
                        path: event.path,
                      );
                    },
                    separatorBuilder: (context, index) =>
                        const SizedBox(height: 8),
                    itemCount: events.length,
                  ),
          ),
        ],
      ),
    );
  }

  void _scrollToLatest() {
    if (!_scrollController.hasClients) {
      return;
    }

    _scrollController.animateTo(
      _scrollController.position.maxScrollExtent,
      duration: const Duration(milliseconds: 160),
      curve: Curves.easeOutCubic,
    );
  }
}

class _FooterRow extends StatelessWidget {
  const _FooterRow({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    final actionLabel = state.running ? '정지' : '검사 시작';
    final actionTone = state.running
        ? AppButtonTone.destructiveSolid
        : AppButtonTone.primary;
    final actionIcon = state.running ? AppIcons.stop : AppIcons.play;

    return SizedBox(
      height: 40,
      child: Row(
        children: [
          Expanded(
            child: SizedBox(
              height: 36,
              child: _Panel(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Row(
                  children: [
                    const Icon(AppIcons.info, size: 16),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _actionHint(state),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          color: AppTokens.mutedForeground,
                          fontSize: 13,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(width: 12),
          SizedBox(
            width: 112,
            child: AppButton(
              label: '캐시 삭제',
              tone: AppButtonTone.destructive,
              icon: AppIcons.trash,
              expanded: true,
              onPressed: state.running
                  ? null
                  : () => controller.openDialog(ScanDialog.cacheConfirm),
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 112,
            child: AppButton(
              label: actionLabel,
              tone: actionTone,
              icon: actionIcon,
              expanded: true,
              onPressed: state.running
                  ? controller.stop
                  : () => _startScan(context),
            ),
          ),
        ],
      ),
    );
  }

  void _startScan(BuildContext context) {
    if (state.quarantineRequired) {
      showFToast(
        context: context,
        variant: FToastVariant.destructive,
        icon: const Icon(AppIcons.info),
        title: const Text('보관 폴더를 먼저 지정하십시오.'),
        description: const Text('검사 설정의 보관 폴더에서 디스크별 보관 위치를 선택한 뒤 다시 시작하십시오.'),
      );
      controller.openDialog(ScanDialog.quarantineSettings);
      return;
    }

    controller.openDialog(ScanDialog.scanConfirm);
  }

  String _actionHint(ScanState state) {
    if (!state.hasPaths) {
      return '중복 파일을 검사할 디렉터리를 선택하십시오.';
    }

    if (state.running) {
      return '검사 및 중복 제거를 실행 중입니다.';
    }

    if (state.quarantineRequired) {
      return '검사 전 모든 디스크의 보관 폴더를 지정하십시오.';
    }

    return '선택한 디렉터리에서 중복 제거를 시작할 수 있습니다.';
  }
}

class _DialogHost extends StatelessWidget {
  const _DialogHost({required this.state, required this.controller});

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    final reducedMotion = MediaQuery.disableAnimationsOf(context);
    final dialog = switch (state.openDialog) {
      ScanDialog.none => const SizedBox.shrink(),
      ScanDialog.cacheConfirm => _ConfirmDialog(
        key: const ValueKey(ScanDialog.cacheConfirm),
        title: 'SQLite 캐시 삭제',
        message:
            'SQLite 해시 캐시를 삭제하면 저장된 파일 해시 기록이 지워져 다음 검사에서 캐시를 재사용하지 않고 필요한 해시를 다시 계산합니다. 진행하시겠습니까?',
        confirmLabel: '예',
        cancelLabel: '아니오',
        destructive: true,
        onCancel: controller.closeDialog,
        onConfirm: controller.clearCache,
      ),
      ScanDialog.scanConfirm => _ConfirmDialog(
        key: const ValueKey(ScanDialog.scanConfirm),
        title: '검사 시작',
        message: '검사 및 중복 제거 작업이 시작됩니다. 정말 진행하시겠습니까?',
        confirmLabel: '예',
        cancelLabel: '아니오',
        onCancel: controller.closeDialog,
        onConfirm: controller.run,
      ),
      ScanDialog.pathList => _PathListDialog(
        key: const ValueKey(ScanDialog.pathList),
        state: state,
        controller: controller,
      ),
      ScanDialog.cacheSettings => _CacheSettingsDialog(
        key: const ValueKey(ScanDialog.cacheSettings),
        state: state,
        controller: controller,
      ),
      ScanDialog.scanModeSettings => _ScanModeDialog(
        key: const ValueKey(ScanDialog.scanModeSettings),
        state: state,
        controller: controller,
      ),
      ScanDialog.algorithmSettings => _AlgorithmDialog(
        key: const ValueKey(ScanDialog.algorithmSettings),
        state: state,
        controller: controller,
      ),
      ScanDialog.quarantineSettings => _QuarantineDialog(
        key: const ValueKey(ScanDialog.quarantineSettings),
        state: state,
        controller: controller,
      ),
      ScanDialog.duplicateRelations => _RelationDialog(
        key: const ValueKey(ScanDialog.duplicateRelations),
        state: state,
        controller: controller,
      ),
    };

    return AnimatedSwitcher(
      duration: reducedMotion
          ? Duration.zero
          : const Duration(milliseconds: 140),
      reverseDuration: reducedMotion
          ? Duration.zero
          : const Duration(milliseconds: 110),
      switchInCurve: Curves.easeOutCubic,
      switchOutCurve: Curves.easeInCubic,
      child: dialog,
    );
  }
}

class _PathListDialog extends StatelessWidget {
  const _PathListDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _Modal(
      width: 560,
      title: '검사 경로',
      message: '삭제할 폴더를 선택한 뒤 선택 삭제를 누르십시오. 서로 다른 디스크의 경로도 함께 검사할 수 있습니다.',
      actions: [
        Expanded(child: Text('${state.pathRemoveSelection.length}개 선택됨')),
        AppButton(
          label: '닫기',
          tone: AppButtonTone.outline,
          onPressed: controller.closeDialog,
        ),
        AppButton(
          label: '선택 삭제',
          tone: AppButtonTone.destructive,
          onPressed: state.pathRemoveSelection.isEmpty
              ? null
              : controller.removeSelectedPaths,
        ),
      ],
      child: _DialogList(
        emptyLabel: '선택된 폴더가 없습니다.',
        itemCount: state.paths.length,
        itemBuilder: (context, index) {
          final path = state.paths[index];

          return AppCheckboxRow(
            value: state.pathRemoveSelection.contains(path),
            onChanged: state.running
                ? null
                : (value) => controller.togglePathSelection(path, value),
            child: Text(path, overflow: TextOverflow.ellipsis),
          );
        },
      ),
    );
  }
}

class _CacheSettingsDialog extends StatelessWidget {
  const _CacheSettingsDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _Modal(
      title: 'SQLite 캐시 제한',
      message:
          '캐시 DB가 입력한 용량을 넘으면 오래된 해시 기록부터 삭제합니다. 제한값은 MB 단위이며 16 이상으로 입력하십시오.',
      actions: [
        AppButton(
          label: '취소',
          tone: AppButtonTone.outline,
          onPressed: controller.closeDialog,
        ),
        AppButton(label: '저장', onPressed: controller.saveCacheLimit),
      ],
      child: AppTextField(
        value: state.cacheLimitInput,
        keyboardType: TextInputType.number,
        onChanged: controller.setCacheLimitInput,
      ),
    );
  }
}

class _ScanModeDialog extends StatelessWidget {
  const _ScanModeDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _OptionDialog<ScanMode>(
      title: '검사 모드',
      message: '파일을 검사 대상으로 선별하는 방식을 선택하십시오. 기본값은 빠른 일반 모드입니다.',
      values: ScanMode.values,
      selected: state.scanMode,
      label: (value) => value.label,
      description: (value) => value.description,
      onSelected: controller.setScanMode,
      onClose: controller.closeDialog,
    );
  }
}

class _AlgorithmDialog extends StatelessWidget {
  const _AlgorithmDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _OptionDialog<HashAlgorithm>(
      title: '비교 기준',
      message: '파일 내용 비교에 사용할 해시 알고리즘을 선택하십시오. 기본값은 BLAKE3입니다.',
      values: HashAlgorithm.values,
      selected: state.algorithm,
      label: (value) => value.label,
      description: (value) => value.description,
      onSelected: controller.setAlgorithm,
      onClose: controller.closeDialog,
    );
  }
}

class _QuarantineDialog extends StatelessWidget {
  const _QuarantineDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _Modal(
      width: 680,
      title: '보관 폴더',
      message:
          '중복 파일은 삭제하지 않고 같은 디스크의 보관 폴더로 이동합니다. 검사 시작 전 디스크별 보관 폴더를 반드시 지정하십시오.',
      actions: [
        AppButton(
          label: '닫기',
          tone: AppButtonTone.outline,
          onPressed: controller.closeDialog,
        ),
      ],
      child: _DialogList(
        emptyLabel: '검사 경로를 먼저 선택하십시오.',
        itemCount: state.quarantineDestinations.length,
        itemBuilder: (context, index) {
          final destination = state.quarantineDestinations[index];

          return _QuarantineDestinationRow(
            destination: destination,
            onPressed: state.running
                ? null
                : () => controller.pickQuarantineDestination(destination),
          );
        },
      ),
    );
  }
}

class _QuarantineDestinationRow extends StatelessWidget {
  const _QuarantineDestinationRow({
    required this.destination,
    required this.onPressed,
  });

  final VolumeDestination destination;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return DecoratedBox(
      decoration: const BoxDecoration(
        border: Border(bottom: BorderSide(color: AppTokens.border)),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Padding(
              padding: EdgeInsets.only(top: 1),
              child: AppIconBox(icon: AppIcons.archive, compact: true),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  for (final root in destination.rootPaths)
                    Text(
                      root,
                      softWrap: true,
                      style: theme.typography.sm.copyWith(
                        color: AppTokens.foreground,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  const SizedBox(height: 6),
                  Text(
                    destination.targetPath,
                    softWrap: true,
                    style: theme.typography.sm.copyWith(
                      color: AppTokens.mutedForeground,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            AppButton(
              label: '폴더 선택',
              tone: AppButtonTone.outline,
              size: AppButtonSize.small,
              onPressed: onPressed,
            ),
          ],
        ),
      ),
    );
  }
}

class _RelationDialog extends StatelessWidget {
  const _RelationDialog({
    required this.state,
    required this.controller,
    super.key,
  });

  final ScanState state;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    final relations =
        state.report?.duplicateRelations ?? const <DuplicateRelation>[];
    final filtered = state.filteredDuplicateRelations;

    return _Modal(
      width: 760,
      title: '중복 관계',
      message: '원본 파일과 보관된 중복 파일의 관계를 비교하고 파일 위치를 열 수 있습니다.',
      actions: [
        AppButton(
          label: '로그 저장',
          tone: AppButtonTone.outline,
          onPressed: relations.isEmpty
              ? null
              : controller.exportDuplicateRelationsLog,
        ),
        AppButton(label: '닫기', onPressed: controller.closeDialog),
      ],
      child: SizedBox(
        height: 430,
        child: Column(
          children: [
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  for (final filter in DuplicateRelationFilter.values)
                    Padding(
                      padding: const EdgeInsets.only(right: 6),
                      child: AppButton(
                        label:
                            '${filter.label} ${relations.where(filter.matches).length}',
                        tone: AppButtonTone.outline,
                        size: AppButtonSize.small,
                        selected: state.duplicateRelationFilter == filter,
                        onPressed: () =>
                            controller.setDuplicateRelationFilter(filter),
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(height: 10),
            Expanded(
              child: filtered.isEmpty
                  ? const AppEmptyState(label: '선택한 필터에 해당하는 중복 관계가 없습니다.')
                  : ListView.separated(
                      itemBuilder: (context, index) {
                        return _RelationCard(
                          index: index,
                          relation: filtered[index],
                          controller: controller,
                        );
                      },
                      separatorBuilder: (context, index) =>
                          const SizedBox(height: 10),
                      itemCount: filtered.length,
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _OptionDialog<T> extends StatelessWidget {
  const _OptionDialog({
    required this.title,
    required this.message,
    required this.values,
    required this.selected,
    required this.label,
    required this.description,
    required this.onSelected,
    required this.onClose,
  });

  final String title;
  final String message;
  final List<T> values;
  final T selected;
  final String Function(T value) label;
  final String Function(T value) description;
  final ValueChanged<T> onSelected;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return _Modal(
      title: title,
      message: message,
      actions: [
        AppButton(label: '닫기', tone: AppButtonTone.outline, onPressed: onClose),
      ],
      child: Column(
        children: [
          for (final value in values)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: AppRadioRow(
                selected: selected == value,
                title: label(value),
                description: description(value),
                onChanged: () => onSelected(value),
              ),
            ),
        ],
      ),
    );
  }
}

class _ConfirmDialog extends StatelessWidget {
  const _ConfirmDialog({
    required this.title,
    required this.message,
    required this.confirmLabel,
    required this.cancelLabel,
    required this.onCancel,
    required this.onConfirm,
    this.destructive = false,
    super.key,
  });

  final String title;
  final String message;
  final String confirmLabel;
  final String cancelLabel;
  final VoidCallback onCancel;
  final VoidCallback onConfirm;
  final bool destructive;

  @override
  Widget build(BuildContext context) {
    return _Modal(
      title: title,
      message: message,
      actions: [
        AppButton(
          label: cancelLabel,
          tone: AppButtonTone.outline,
          onPressed: onCancel,
        ),
        AppButton(
          label: confirmLabel,
          tone: destructive ? AppButtonTone.destructive : AppButtonTone.primary,
          onPressed: onConfirm,
        ),
      ],
    );
  }
}

class _Modal extends StatelessWidget {
  const _Modal({
    required this.title,
    this.message,
    this.child,
    this.actions = const [],
    this.width = 420,
  });

  final String title;
  final String? message;
  final Widget? child;
  final List<Widget> actions;
  final double width;

  @override
  Widget build(BuildContext context) {
    return AppDialog(
      title: title,
      message: message,
      actions: actions,
      width: width,
      child: child,
    );
  }
}

class _DialogList extends StatelessWidget {
  const _DialogList({
    required this.emptyLabel,
    required this.itemCount,
    required this.itemBuilder,
  });

  final String emptyLabel;
  final int itemCount;
  final IndexedWidgetBuilder itemBuilder;

  @override
  Widget build(BuildContext context) {
    if (itemCount == 0) {
      return AppEmptyState(label: emptyLabel);
    }

    return ConstrainedBox(
      constraints: const BoxConstraints(maxHeight: 320),
      child: ListView.separated(
        shrinkWrap: true,
        itemBuilder: itemBuilder,
        separatorBuilder: (context, index) => const SizedBox(height: 6),
        itemCount: itemCount,
      ),
    );
  }
}

class _RelationCard extends StatelessWidget {
  const _RelationCard({
    required this.index,
    required this.relation,
    required this.controller,
  });

  final int index;
  final DuplicateRelation relation;
  final ScanController controller;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      padding: const EdgeInsets.all(10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text('관계 ${index + 1}'),
              const Spacer(),
              Text(formatBytes(relation.size)),
              const SizedBox(width: 8),
              Text(compactHashLabel(relation.hash)),
            ],
          ),
          const SizedBox(height: 8),
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  child: _RelationFile(
                    label: '원본 파일',
                    path: relation.originalPath,
                    onOpen: () =>
                        controller.revealFileLocation(relation.originalPath),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _RelationFile(
                    label: '중복 파일',
                    path: relation.duplicatePath,
                    muted: '보관 위치: ${relation.currentDuplicatePath}',
                    onOpen: () => controller.revealFileLocation(
                      relation.currentDuplicatePath,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _RelationFile extends StatelessWidget {
  const _RelationFile({
    required this.label,
    required this.path,
    required this.onOpen,
    this.muted,
  });

  final String label;
  final String path;
  final String? muted;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return _Panel(
      padding: const EdgeInsets.all(10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: theme.typography.sm.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 6),
          Text(path, maxLines: 1, overflow: TextOverflow.ellipsis),
          if (muted != null) ...[
            const SizedBox(height: 6),
            Text(
              muted!,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.typography.sm.copyWith(
                color: theme.colors.mutedForeground,
              ),
            ),
          ],
          const SizedBox(height: 6),
          const Spacer(),
          SizedBox(
            width: double.infinity,
            child: AppButton(
              label: '위치 열기',
              tone: AppButtonTone.outline,
              size: AppButtonSize.small,
              expanded: true,
              onPressed: onOpen,
            ),
          ),
        ],
      ),
    );
  }
}

class _SettingRow extends StatelessWidget {
  const _SettingRow({
    required this.icon,
    required this.label,
    required this.value,
    required this.disabled,
    required this.onPressed,
    this.showBottomBorder = true,
  });

  final IconData icon;
  final String label;
  final String value;
  final bool disabled;
  final VoidCallback onPressed;
  final bool showBottomBorder;

  @override
  Widget build(BuildContext context) {
    return AppItemRow(
      icon: icon,
      title: Text(label),
      showBottomBorder: showBottomBorder,
      details: SizedBox(
        width: 120,
        child: Text(
          value,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          textAlign: TextAlign.right,
          style: const TextStyle(
            color: AppTokens.foreground,
            fontSize: 13,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
      suffix: AppButton(
        label: '설정',
        tone: AppButtonTone.outline,
        size: AppButtonSize.small,
        onPressed: disabled ? null : onPressed,
      ),
    );
  }
}

class _MiniStat extends StatelessWidget {
  const _MiniStat({
    required this.icon,
    required this.label,
    required this.value,
  });

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 48,
      child: Center(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 16, color: AppTokens.foreground),
            const SizedBox(width: 10),
            Flexible(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AppTokens.mutedForeground,
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  Text(
                    value,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AppTokens.foreground,
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ResultRow extends StatelessWidget {
  const _ResultRow({
    required this.icon,
    required this.tone,
    required this.label,
    required this.value,
  });

  final IconData icon;
  final AppIconTone tone;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return SizedBox(
      height: 58,
      child: _Panel(
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: Row(
          children: [
            AppIconBox(icon: icon, tone: tone, size: AppIconBoxSize.result),
            const SizedBox(width: 10),
            Expanded(
              child: Row(
                children: [
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.typography.sm.copyWith(
                        color: AppTokens.mutedForeground,
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  const SizedBox(width: 6),
                  Flexible(
                    flex: 2,
                    child: Align(
                      alignment: Alignment.centerRight,
                      child: FittedBox(
                        fit: BoxFit.scaleDown,
                        alignment: Alignment.centerRight,
                        child: Text(
                          value,
                          maxLines: 1,
                          softWrap: false,
                          style: const TextStyle(
                            color: AppTokens.foreground,
                            fontSize: 19,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ActivityEmpty extends StatelessWidget {
  const _ActivityEmpty();

  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.all(12),
      child: _ActivityRow(
        stage: '대기 중',
        detail: '검사를 시작하면 현재 처리 중인 작업과 파일이 표시됩니다.',
        idle: true,
      ),
    );
  }
}

class _ActivityRow extends StatelessWidget {
  const _ActivityRow({
    required this.stage,
    required this.detail,
    this.path,
    this.idle = false,
  });

  final String stage;
  final String detail;
  final String? path;
  final bool idle;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          _ActivityDot(idle: idle),
          const SizedBox(width: 10),
          Expanded(
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: '$stage  ',
                    style: const TextStyle(
                      color: AppTokens.mutedForeground,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  TextSpan(text: detail),
                  if (path != null) TextSpan(text: ' $path'),
                ],
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(
                color: AppTokens.mutedForeground,
                fontSize: 13,
                fontWeight: FontWeight.w400,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ActivityDot extends StatelessWidget {
  const _ActivityDot({required this.idle});

  final bool idle;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 14,
      height: 14,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: idle ? AppTokens.blueSoft : AppTokens.background,
          shape: const CircleBorder(),
        ),
        child: const Center(
          child: SizedBox(
            width: 7,
            height: 7,
            child: DecoratedBox(
              decoration: ShapeDecoration(
                color: AppTokens.primary,
                shape: CircleBorder(),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({
    required this.child,
    this.padding = const EdgeInsets.all(12),
    this.backgroundColor = AppTokens.background,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    return AppPanel(
      padding: padding,
      backgroundColor: backgroundColor,
      child: child,
    );
  }
}
